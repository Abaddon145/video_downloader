import type {CommandDescriptor,CommandScope,CommandStorage} from './CommandRegistry.ts';
import {CommandRegistry} from './CommandRegistry.ts';
export type ShortcutBindings=Record<string,string|null>;
const KEY='yingliu.shortcuts.v1';
const aliases:Record<string,string>={control:'Ctrl',ctrl:'Ctrl',alt:'Alt',option:'Alt',shift:'Shift',meta:'Meta',cmd:'Meta',command:'Meta',space:'Space',' ':'Space',escape:'Escape',esc:'Escape',enter:'Enter',return:'Enter',tab:'Tab',arrowleft:'ArrowLeft',arrowright:'ArrowRight',arrowup:'ArrowUp',arrowdown:'ArrowDown',backspace:'Backspace',delete:'Delete',home:'Home',end:'End',pageup:'PageUp',pagedown:'PageDown',plus:'Plus'};
export function normalizeShortcut(value:string):string|null {
 if(typeof value!=='string'||value.length>64)return null;
 const parts=value.split('+').map(s=>s.trim()).map(s=>aliases[s.toLowerCase()]??(/^[a-z0-9]$/i.test(s)?s.toUpperCase():/^F([1-9]|1[0-2])$/i.test(s)?s.toUpperCase():null));
 if(parts.some(p=>!p)||new Set(parts).size!==parts.length)return null;
 const modifiers=['Ctrl','Alt','Shift','Meta'].filter(m=>parts.includes(m)),keys=parts.filter(p=>!['Ctrl','Alt','Shift','Meta'].includes(p!));
 return keys.length===1?[...modifiers,keys[0]].join('+'):null;
}
export function shortcutIssue(shortcut:string,scope:CommandScope='global',bindings:Pick<CommandDescriptor,'id'|'shortcut'|'scope'>[]=[],id=''):string|null {
 const value=normalizeShortcut(shortcut);if(!value)return '快捷键格式无效';
 if(value==='Tab'||value==='Shift+Tab')return '此快捷键保留给焦点导航';
 if(/^(Ctrl|Meta)\+(Shift\+)?(Z|Y|A|C|V|X|B|I|U|F|H|ArrowLeft|ArrowRight|Home|End)$/.test(value)||value==='Shift+F10')return '此快捷键保留给编辑器或上下文菜单';
 if(['Alt+F4','Alt+Tab','Alt+Shift+Tab','Ctrl+Alt+Delete','Ctrl+W','Ctrl+R','Ctrl+Shift+I','F5','Meta+L','Meta+D','Meta+Tab'].includes(value)||value.startsWith('Meta+'))return '此快捷键保留给系统';
 if(scope==='global'&&!/^(Ctrl|Alt|Meta)\+/.test(value)&&value!=='F11')return '全局快捷键需要 Ctrl 或 Alt 修饰键';
 const conflict=bindings.find(c=>c.id!==id&&c.shortcut&&normalizeShortcut(c.shortcut)===value&&((c.scope??'global')==='global'||scope==='global'||c.scope===scope));
 return conflict?`与 ${conflict.id} 的快捷键冲突`:null;
}
export function importShortcuts(text:string,commands:CommandDescriptor[]):ShortcutBindings {
 if(new TextEncoder().encode(text).length>128*1024)throw new Error('快捷键配置不得超过 128 KiB');
 const data:unknown=JSON.parse(text);
 if(!data||typeof data!=='object'||Array.isArray(data)||Object.keys(data).sort().join(',')!=='bindings,version'||(data as {version:unknown}).version!==1)throw new Error('快捷键配置版本或字段无效');
 const raw=(data as {bindings:unknown}).bindings;
 if(!raw||typeof raw!=='object'||Array.isArray(raw)||Object.keys(raw).length>500)throw new Error('快捷键绑定无效');
 const result:ShortcutBindings=Object.create(null);
 for(const [id,value] of Object.entries(raw)){
  const command=commands.find(c=>c.id===id);if(!command||!/^[a-zA-Z0-9._-]{1,100}$/.test(id)||!(value===null||typeof value==='string'))throw new Error(`未知或无效的命令：${id}`);
  if(value===null){result[id]=null;continue;}const normalized=normalizeShortcut(value);if(!normalized)throw new Error('快捷键格式无效');result[id]=normalized;
 }
 const actual=commands.map(c=>({...c,shortcut:Object.hasOwn(result,c.id)?result[c.id]:c.shortcut}));
 for(const command of actual){if(!command.shortcut)continue;const issue=shortcutIssue(command.shortcut,command.scope,actual,command.id);if(issue)throw new Error(issue);}
 return result;
}
export interface ShortcutEvent {key:string;code?:string;ctrlKey:boolean;altKey:boolean;shiftKey:boolean;metaKey:boolean;isComposing?:boolean;repeat?:boolean;defaultPrevented?:boolean;keyCode?:number}
export function eventShortcut(event:ShortcutEvent){const key=event.shiftKey&&/^Digit[0-9]$/.test(event.code??'')?event.code!.slice(5):event.key;return normalizeShortcut([...event.ctrlKey?['Ctrl']:[],...event.altKey?['Alt']:[],...event.shiftKey?['Shift']:[],...event.metaKey?['Meta']:[],key===' '?'Space':key==='+'?'Plus':key].join('+'));}
export function shouldHandleShortcut(event:ShortcutEvent,command:CommandDescriptor,context:{scope:CommandScope;editable:boolean;overlay:boolean;interactive?:boolean}){
 if(event.defaultPrevented||event.repeat||event.isComposing||event.keyCode===229||context.overlay||command.enabled===false)return false;
 if((command.scope??'global')!=='global'&&command.scope!==context.scope)return false;
 if(context.editable&&!command.allowInEditable)return false;
 if(context.interactive&&['Space','Enter'].includes(eventShortcut(event)??''))return false;
 return !!command.shortcut&&eventShortcut(event)===normalizeShortcut(command.shortcut)&&!shortcutIssue(command.shortcut,command.scope,[],command.id);
}
export class ShortcutManager {
 private registry:CommandRegistry;private storage?:CommandStorage;private overrides:ShortcutBindings=Object.create(null);private listeners=new Set<()=>void>();private target:Pick<Window,'addEventListener'|'removeEventListener'>|null=null;private users=0;
 constructor(registry:CommandRegistry,storage?:CommandStorage){this.registry=registry;this.storage=storage;try{const raw=storage?.getItem(KEY);if(raw&&new TextEncoder().encode(raw).length<=128*1024){const value=JSON.parse(raw);if(value?.version===1&&value.bindings&&typeof value.bindings==='object'&&!Array.isArray(value.bindings)&&Object.keys(value).sort().join(',')==='bindings,version'){for(const [id,binding] of Object.entries(value.bindings))if(/^[a-zA-Z0-9._-]{1,100}$/.test(id)&&(binding===null||typeof binding==='string'&&normalizeShortcut(binding)))this.overrides[id]=binding as string|null;}}}catch{/* Invalid preferences fall back to defaults. */}}
 subscribe=(listener:()=>void)=>{this.listeners.add(listener);return()=>{this.listeners.delete(listener);};};
 getSnapshot=()=>this.overrides;
 binding=(command:CommandDescriptor)=>Object.hasOwn(this.overrides,command.id)?this.overrides[command.id]:command.shortcut??null;
 commands=()=>this.registry.getSnapshot().commands.map(c=>({...c,shortcut:this.binding(c)}));
 issue=(id:string,value:string)=>{const command=this.commands().find(c=>c.id===id);return !command?'命令当前未注册':shortcutIssue(value,command.scope,this.commands(),id);};
 private save(next:ShortcutBindings){this.storage?.setItem(KEY,JSON.stringify({version:1,bindings:next}));this.overrides=next;for(const listener of this.listeners)listener();}
 setBinding=(id:string,value:string|null)=>{if(value!==null){const issue=this.issue(id,value);if(issue)throw new Error(issue);}else if(!this.registry.getSnapshot().commands.some(c=>c.id===id))throw new Error('命令当前未注册');this.save({...this.overrides,[id]:value===null?null:normalizeShortcut(value)});};
 restoreDefaults=()=>this.save({});
 resetBinding=(id:string)=>{const next={...this.overrides};delete next[id];const command=this.registry.getSnapshot().commands.find(c=>c.id===id);if(command?.shortcut){const issue=shortcutIssue(command.shortcut,command.scope,this.commands(),id);if(issue)throw new Error(issue);}this.save(next);};
 import=(text:string)=>this.save(importShortcuts(text,this.registry.getSnapshot().commands));
 export=()=>JSON.stringify({version:1,bindings:Object.fromEntries(Object.entries(this.overrides).filter(([id])=>this.registry.getSnapshot().commands.some(c=>c.id===id)))},null,2);
 private onKeyDown=(event:Event)=>{
  const key=event as KeyboardEvent,target=key.target as HTMLElement|null;
  const overlay=!!target?.closest?.('[role="dialog"],[role="menu"],[data-native-overlay]')||!!(typeof document!=='undefined'&&document.querySelector('dialog[open],[aria-modal="true"],[role="menu"][data-native-overlay]'));
  const editable=!!target?.closest?.('input,textarea,select,[contenteditable]:not([contenteditable="false"]),[role="textbox"],.cm-editor');
  const rawScope=target?.closest?.('[data-command-scope]')?.getAttribute('data-command-scope');const scope:CommandScope=rawScope==='player'||rawScope==='knowledge'?rawScope:'global';
  const interactive=!!target?.closest?.('button,[role="button"],a[href]');
  const actual=this.commands(),matches=actual.filter(c=>shouldHandleShortcut(key,c,{scope,editable,overlay,interactive}));
  if(matches.length!==1)return;const command=matches[0];if(shortcutIssue(command.shortcut!,command.scope,actual,command.id))return;
  key.preventDefault();void this.registry.run(command.id);
 };
 attach=(target:Pick<Window,'addEventListener'|'removeEventListener'>)=>{
  if(this.target&&this.target!==target)throw new Error('快捷键管理器已连接');if(!this.users++){this.target=target;target.addEventListener('keydown',this.onKeyDown);}
  let live=true;return()=>{if(!live)return;live=false;if(!--this.users){target.removeEventListener('keydown',this.onKeyDown);this.target=null;}};
 };
}
