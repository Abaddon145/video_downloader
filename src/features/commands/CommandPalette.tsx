import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import type {KeyboardEvent} from 'react';
import {createPortal} from 'react-dom';
import {searchCommands} from './CommandRegistry';
import {eventShortcut} from './ShortcutManager';
import {useCommands,useCommandSnapshot,useShortcutBindings} from './CommandProvider';
let commandReturnFocus:HTMLElement|null=null;
export function useCommandDialog(open:boolean,onClose:()=>void){
 const dialog=useRef<HTMLDialogElement>(null);
 useLayoutEffect(()=>{
  if(!open)return;const previous=document.activeElement as HTMLElement|null,element=dialog.current;
  if(previous&&previous!==document.body&&!previous.closest('.command-dialog'))commandReturnFocus=previous;
  if(element&&!element.open)element.showModal();
  return()=>{element?.close();queueMicrotask(()=>{if(!document.querySelector('dialog[open],[aria-modal="true"]')){if(commandReturnFocus?.isConnected)commandReturnFocus.focus();commandReturnFocus=null;}});};
 },[open]);
 return {dialog,onCancel:(event:{preventDefault:()=>void})=>{event.preventDefault();onClose();}};
}
export function CommandPalette(){
 const {overlay,closeOverlay,closePalette,shortcuts,run,openHelp}=useCommands(),snapshot=useCommandSnapshot();useShortcutBindings();
 const [query,setQuery]=useState(''),[category,setCategory]=useState(''),[active,setActive]=useState(0),[error,setError]=useState('');
 const input=useRef<HTMLInputElement>(null),session=useRef(0),open=overlay==='palette';
 const {dialog,onCancel}=useCommandDialog(open,closeOverlay);
 useLayoutEffect(()=>{session.current++;if(open){setQuery('');setCategory('');setActive(0);setError('');input.current?.focus();}},[open]);
 const commands=searchCommands(snapshot.commands,query,snapshot.recent,category),selected=commands[Math.min(active,Math.max(0,commands.length-1))];
 const categories=[...new Set(snapshot.commands.map(c=>c.category))];
 useEffect(()=>{dialog.current?.querySelector('[data-active="true"]')?.scrollIntoView({block:'nearest'});},[active,query,category]);
 const execute=async(id:string)=>{const token=session.current;setError('');const result=await run(id);if(session.current!==token)return;if(result.ok)closePalette();else setError(result.error);};
 const onKeyDown=(event:KeyboardEvent)=>{
  if(event.nativeEvent.isComposing||event.keyCode===229)return;
  const palette=snapshot.commands.find(c=>c.id==='app.palette');
  if(palette&&eventShortcut(event.nativeEvent)===shortcuts.binding(palette)){event.preventDefault();closeOverlay();return;}
  if(event.key==='Escape'){event.preventDefault();closeOverlay();return;}
  if((event.target as HTMLElement).tagName==='SELECT')return;
  if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();setActive(old=>commands.length?(old+(event.key==='ArrowDown'?1:-1)+commands.length)%commands.length:0);}
  if(event.key==='Enter'&&((event.target as HTMLElement).tagName==='INPUT'||(event.target as HTMLElement).getAttribute('role')==='listbox')){event.preventDefault();if(selected)void execute(selected.id);}
 };
 if(!open)return null;
 return createPortal(<dialog ref={dialog} className="command-dialog" data-native-overlay="true" aria-labelledby="command-palette-title" onCancel={onCancel} onKeyDown={onKeyDown} onClick={event=>{if(event.target===event.currentTarget)closeOverlay();}}>
  <header><h2 id="command-palette-title">命令面板</h2><button type="button" aria-label="关闭命令面板" onClick={closeOverlay}>×</button></header>
  <input ref={input} className="command-query" type="search" role="combobox" aria-label="搜索命令" aria-expanded="true" aria-controls="command-results" aria-autocomplete="list" aria-activedescendant={selected?`command-result-${selected.id}`:undefined} value={query} placeholder="搜索命令、中文或 English…" onChange={event=>{setQuery(event.target.value);setActive(0);setError('');}}/>
  <div className="command-filter"><label>分类 <select value={category} onChange={event=>{setCategory(event.target.value);setActive(0);}}><option value="">全部命令</option>{categories.map(c=><option key={c}>{c}</option>)}</select></label><span>{!query&&!category?'最近使用与常用命令':`${commands.length} 个结果`}</span></div>
  <div id="command-results" className="command-results" role="listbox" aria-label="命令结果" tabIndex={0} aria-activedescendant={selected?`command-result-${selected.id}`:undefined}>
   {!commands.length&&<p className="command-empty">没有匹配的命令。试试名称、分类或英文关键词。</p>}
   {commands.map((command,index)=>{const running=snapshot.running.includes(command.id),disabled=command.enabled===false||running,binding=shortcuts.binding(command),issue=binding?shortcuts.issue(command.id,binding):null;
    return <div key={command.id} id={`command-result-${command.id}`} role="option" aria-selected={selected?.id===command.id} aria-disabled={disabled} data-active={selected?.id===command.id} className="command-result" onMouseMove={()=>setActive(index)} onClick={()=>{if(!disabled)void execute(command.id);}}>
     <div><strong>{command.title}</strong><small>{command.category}{command.description?` · ${command.description}`:''}</small>{disabled&&<small>{running?'正在执行…':command.availabilityReason||'当前不可用'}</small>}{issue&&<small>{issue}</small>}</div><kbd>{binding??'未绑定'}</kbd>
    </div>;
   })}
  </div>
  {error&&<p className="command-error" role="alert">{error}</p>}
  {snapshot.running.length>0&&<p role="status">正在执行…</p>}
  <footer><span>↑↓ 选择 · Enter 执行 · Tab 切换区域 · Esc 关闭</span><button type="button" onClick={openHelp}>快捷键帮助</button></footer>
 </dialog>,document.body);
}

