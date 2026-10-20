import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {invoke} from '@tauri-apps/api/core';
import type {LearningItem} from '../../types/learning';
import type {KnowledgeSettings,KnowledgeView,SearchPage,KnowledgeReference} from '../../types/knowledge';
import {useCommandDialog} from '../commands/CommandPalette';
interface Courses{items:LearningItem[];total:number;page:number}
export function QuickSearch({open,close,onVideo,onNote}:{open:boolean;close:()=>void;onVideo:(id:string,time:number)=>void;onNote:(r:KnowledgeReference)=>void}){
 const [query,setQuery]=useState(''),[page,setPage]=useState(1),[items,setItems]=useState<Courses|null>(null),[notes,setNotes]=useState<SearchPage|null>(null),[vault,setVault]=useState<string|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false),[active,setActive]=useState(0);
 const input=useRef<HTMLInputElement>(null),{dialog,onCancel}=useCommandDialog(open,close);
 useLayoutEffect(()=>{if(open){setQuery('');setPage(1);setActive(0);input.current?.focus();}},[open]);
 useEffect(()=>{if(!open)return;let alive=true;const timer=setTimeout(()=>{setBusy(true);setError('');void (async()=>{
  const course=await invoke<Courses>('learning_search_items',{query,page});
  if(!alive)return;setItems(course);
  const settings=await invoke<KnowledgeSettings>('knowledge_get_settings');if(!alive)return;setVault(settings.vaultId);
  if(settings.vaultId&&query.trim()){const result=await invoke<KnowledgeView<SearchPage>>('knowledge_search',{vaultId:settings.vaultId,query,page,tag:null});if(alive&&result.vaultId===settings.vaultId)setNotes(result.data);}else if(alive)setNotes(null);
 })().catch(e=>{if(alive)setError(String(e));}).finally(()=>{if(alive)setBusy(false);});},180);return()=>{alive=false;clearTimeout(timer);};},[open,query,page]);
 const rows=[...(items?.items??[]).map(item=>({key:`course-${item.id}`,title:item.title,description:`课程 · ${item.available?'可播放':'需下载或重新定位'}`,run:()=>onVideo(item.id,item.progress.positionMs)})),...(notes?.items??[]).map((hit,i)=>({key:`note-${i}`,title:hit.title,description:hit.kind==='transcript'?'课程字幕':'知识笔记',run:()=>{if(hit.kind==='note'&&vault&&hit.path)onNote({vaultId:vault,path:hit.path});else if(hit.itemId)onVideo(hit.itemId,hit.startMs??0);}}))];
 const run=(at:number)=>{if(busy||error)return;const row=rows[at];if(row){close();row.run();}};
 if(!open)return null;
 return createPortal(<dialog ref={dialog} className="command-dialog" data-native-overlay="true" aria-labelledby="quick-search-title" onCancel={onCancel} onKeyDown={e=>{if(e.nativeEvent.isComposing)return;if(e.key==='ArrowDown'||e.key==='ArrowUp'){e.preventDefault();setActive(n=>rows.length?(n+(e.key==='ArrowDown'?1:-1)+rows.length)%rows.length:0);}if(e.key==='Enter'&&e.target===input.current){e.preventDefault();run(active);}}}><header><h2 id="quick-search-title">搜索课程与笔记</h2><button onClick={close} aria-label="关闭全局搜索">×</button></header><input ref={input} aria-label="搜索课程与知识笔记" className="command-query" maxLength={200} placeholder="输入课程、知识或字幕关键词…" value={query} onChange={e=>{setQuery(e.target.value);setPage(1);setActive(0);}}/>{busy&&<p role="status">正在搜索本地内容…</p>}{error&&<p role="alert">搜索未完成，已有数据保留。{error}</p>}<div className="command-results">{!busy&&!rows.length&&<p>没有找到结果。可更换关键词，或先添加课程和知识笔记。</p>}{rows.map((row,i)=><button className="command-result" key={row.key} data-active={active===i} disabled={busy||!!error} onClick={()=>run(i)}><div><strong>{row.title}</strong><small>{row.description}</small></div></button>)}</div><footer><button className="secondary" disabled={page<=1||busy} onClick={()=>{setPage(n=>n-1);setActive(0);}}>上一页</button><span>第 {page} 页 · 每类最多50条</span><button className="secondary" disabled={busy||page*50>=Math.max(items?.total??0,notes?.total??0)} onClick={()=>{setPage(n=>n+1);setActive(0);}}>下一页</button></footer></dialog>,document.body);
}
