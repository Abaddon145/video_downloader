import {useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {useCommands,useCommandSnapshot,useShortcutBindings} from './CommandProvider';
import {useCommandDialog} from './CommandPalette';
import {eventShortcut} from './ShortcutManager';
import {searchCommands} from './CommandRegistry';
export function ShortcutSettings(){
 const {overlay,closeOverlay,shortcuts}=useCommands(),snapshot=useCommandSnapshot();useShortcutBindings();
 const [query,setQuery]=useState(''),[editing,setEditing]=useState(''),[draft,setDraft]=useState(''),[error,setError]=useState(''),[status,setStatus]=useState('');
 const file=useRef<HTMLInputElement>(null),open=overlay==='help';const {dialog,onCancel}=useCommandDialog(open,closeOverlay);
 const commands=searchCommands(snapshot.commands,query),categories=[...new Set(commands.map(c=>c.category))];
 const perform=(action:()=>void,message:string)=>{try{action();setError('');setStatus(message);}catch(error){setStatus('');setError(error instanceof Error?error.message:String(error));}};
 const exportFile=()=>perform(()=>{const url=URL.createObjectURL(new Blob([shortcuts.export()],{type:'application/json'})),link=document.createElement('a');link.href=url;link.download='yingliu-shortcuts.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);},'已导出快捷键配置');
 if(!open)return null;
 return createPortal(<dialog ref={dialog} className="command-dialog command-shortcuts" data-native-overlay="true" aria-labelledby="shortcut-settings-title" onCancel={onCancel} onKeyDown={event=>{if(event.nativeEvent.isComposing)return;if(event.key==='Escape'){event.preventDefault();closeOverlay();}}}>
  <header><h2 id="shortcut-settings-title">快捷键帮助与设置</h2><button type="button" aria-label="关闭快捷键设置" onClick={closeOverlay}>×</button></header>
  <input autoFocus className="command-query" aria-label="搜索快捷键" placeholder="搜索命令或模块…" value={query} onChange={event=>setQuery(event.target.value)}/>
  <div className="shortcut-tools"><button type="button" onClick={()=>perform(shortcuts.restoreDefaults,'已恢复全部默认快捷键')}>恢复默认</button><button type="button" onClick={()=>file.current?.click()}>导入 JSON</button><button type="button" onClick={exportFile}>导出 JSON</button><input ref={file} type="file" accept=".json,application/json" hidden onChange={async event=>{const selected=event.target.files?.[0];event.target.value='';if(!selected)return;if(selected.size>128*1024){setError('快捷键配置不得超过 128 KiB');return;}try{const text=await selected.text();perform(()=>shortcuts.import(text),'已导入快捷键配置');}catch(error){setError(String(error));}}}/></div>
  <p className="shortcut-note">播放器按键仅在播放器获得焦点时生效。输入法、文本编辑和弹层优先；Ctrl+Z 始终保留给编辑器。</p>
  <div className="shortcut-groups">{categories.map(category=><section key={category}><h3>{category}</h3>{commands.filter(c=>c.category===category).map(command=>{const binding=shortcuts.binding(command),issue=binding?shortcuts.issue(command.id,binding):null;
   return <article key={command.id}><div><strong>{command.title}</strong><small>{command.scope==='player'?'播放器焦点':command.scope==='knowledge'?'知识库':'全局'}{command.enabled===false?` · ${command.availabilityReason||'当前不可用'}`:''}</small>{issue&&<small className="command-error">{issue}</small>}</div>
    {editing===command.id?<form onSubmit={event=>{event.preventDefault();perform(()=>{shortcuts.setBinding(command.id,draft||null);setEditing('');},'已保存快捷键');}}><input autoFocus aria-label={`${command.title} 的快捷键`} value={draft} placeholder="例如 Ctrl+Alt+N" onChange={event=>setDraft(event.target.value)} onKeyDown={event=>{if(event.nativeEvent.isComposing)return;if(event.key==='Tab'||event.key==='Enter'||['Control','Alt','Shift','Meta'].includes(event.key))return;if(event.key==='Escape'){event.preventDefault();event.stopPropagation();setEditing('');return;}event.preventDefault();const value=eventShortcut(event.nativeEvent);if(value)setDraft(value);}}/><button type="submit">保存</button><button type="button" onClick={()=>setDraft('')}>清除</button><button type="button" onClick={()=>setEditing('')}>取消</button></form>:<><kbd>{binding??'未绑定'}</kbd><button type="button" onClick={()=>{setEditing(command.id);setDraft(binding??'');setError('');}}>修改</button><button type="button" onClick={()=>perform(()=>shortcuts.resetBinding(command.id),'已恢复此命令的默认快捷键')}>默认</button></>}
   </article>;
  })}</section>)}{!commands.length&&<p>没有匹配的快捷键。</p>}</div>
  {error&&<p className="command-error" role="alert">{error}</p>}{status&&<p role="status">{status}</p>}
 </dialog>,document.body);
}
