import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {KnowledgeDocument,KnowledgeMutation,KnowledgeReference,KnowledgeSettings,KnowledgeView,KnowledgeWrite,SearchPage} from '../../types/knowledge';
import {useTaskCenter} from '../tasks/TaskCenter';
import {useFeedback} from '../../components/Feedback';
import {LearningDialog} from '../learning/Editors';
import {aiError} from './lib';
import {appendKnowledge,safeNoteName} from './courseKnowledge';

export function CourseKnowledgeDraft({itemId,title,markdown,close,onOpenKnowledge,onKnowledgeSetup}:{itemId:string;title:string;markdown:string;close:()=>void;onOpenKnowledge?:(reference:KnowledgeReference)=>void;onKnowledgeSetup?:()=>void}){
 const {trackOperation}=useTaskCenter(),{recordUndo}=useFeedback();
 const [settings,setSettings]=useState<KnowledgeSettings|null>(null),[mode,setMode]=useState<'create'|'append'>('create'),[path,setPath]=useState(''),[target,setTarget]=useState<KnowledgeDocument|null>(null),[search,setSearch]=useState(''),[results,setResults]=useState<SearchPage|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const ticket=useRef(0),locked=useRef(false),item=useRef(itemId);item.current=itemId;
 useEffect(()=>{const own=++ticket.current;setSettings(null);setTarget(null);setError('');invoke<KnowledgeSettings>('knowledge_get_settings').then(value=>{if(ticket.current===own){setSettings(value);setPath(`${value.rootFolder||'映流'}/${safeNoteName(title)}`);}}).catch(e=>{if(ticket.current===own)setError(aiError(e));});return()=>{++ticket.current;};},[itemId,title]);
 const vaultId=settings?.vaultId;
 async function readTarget(selectedPath=path){if(!vaultId||!selectedPath.trim()||locked.current)return;const own=++ticket.current;setBusy(true);setError('');setTarget(null);try{const result=await invoke<KnowledgeView<KnowledgeDocument>>('knowledge_read',{vaultId,path:selectedPath.trim()});if(ticket.current===own&&result.vaultId===vaultId){setPath(result.data.path);setTarget(result.data);}}catch(e){if(ticket.current===own)setError(aiError(e));}finally{if(ticket.current===own)setBusy(false);}}
 async function findNotes(page=1){if(!vaultId||!search.trim()||locked.current)return;const own=++ticket.current;setBusy(true);setError('');try{const result=await invoke<KnowledgeView<SearchPage>>('knowledge_search',{vaultId,query:search.trim(),page,tag:null});if(ticket.current===own&&result.vaultId===vaultId)setResults(result.data);}catch(e){if(ticket.current===own)setError(aiError(e));}finally{if(ticket.current===own)setBusy(false);}}
 const proposed=mode==='create'?markdown:target?appendKnowledge(target.text,markdown,target.hash,target.hash):null;
 async function save(){if(locked.current||!vaultId||!proposed||item.current!==itemId)return;locked.current=true;const own=++ticket.current;setBusy(true);setError('');try{
  const currentSettings=await invoke<KnowledgeSettings>('knowledge_get_settings');if(own!==ticket.current)return;if(currentSettings.vaultId!==vaultId)throw Error('知识库已切换，请重新确认目标');
  if(mode==='create'){
   const hash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(proposed)))].map(byte=>byte.toString(16).padStart(2,'0')).join('');
   const mutation=await trackOperation('note','创建课程摘录',async()=>{const result=await invoke<KnowledgeMutation>('knowledge_create',{vaultId,path:path.trim(),text:proposed,directory:false});if(result.vaultId!==vaultId||!result.path)throw Error('新笔记回执与确认目标不一致');return result;});
   recordUndo({label:'创建课程摘录',undo:()=>invoke('knowledge_trash_note',{vaultId,path:mutation.path,expectedHash:hash})});
   if(own!==ticket.current)return;
   onOpenKnowledge?.({vaultId,path:mutation.path});close();
  }else if(target){
   const latest=(await invoke<KnowledgeView<KnowledgeDocument>>('knowledge_read',{vaultId,path:target.path})).data;
   if(own!==ticket.current)return;
   if((await invoke<KnowledgeSettings>('knowledge_get_settings')).vaultId!==vaultId)throw Error('知识库已切换，草稿仍在；请重新确认目标');
   const text=appendKnowledge(latest.text,markdown,target.hash,latest.hash);
   if(text===null)throw Error('目标笔记已被修改，草稿仍在；请重新读取目标后确认');
   const result=await trackOperation('note','追加课程摘录',async()=>{const write=await invoke<KnowledgeWrite>('knowledge_write',{vaultId,path:target.path,text,expectedHash:target.hash});if(write.vaultId!==vaultId||write.outcome.status!=='saved'||write.outcome.document.path!==target.path)throw Error('目标笔记已被修改，草稿仍在；请重新读取目标后确认');return write;});
   const savedHash=result.outcome.document.hash,original=target.text,savedPath=target.path;
   recordUndo({label:'追加课程摘录',undo:async()=>{const undone=await invoke<KnowledgeWrite>('knowledge_write',{vaultId,path:savedPath,text:original,expectedHash:savedHash});if(undone.outcome.status!=='saved')throw Error('笔记已发生后续修改，撤销不会覆盖这些内容');}});
   if(own!==ticket.current)return;
   onOpenKnowledge?.({vaultId,path:savedPath});close();
  }
 }catch(e){if(own===ticket.current)setError(aiError(e));}finally{if(own===ticket.current){locked.current=false;setBusy(false);}}}
 return <LearningDialog title="保存到知识库" close={close} busy={busy} className="course-knowledge-draft"><p className="help">确认下方完整 Markdown 和保存目标后才会写入知识库。</p>
  {!vaultId?<p className="error-text">尚未选择知识库。{onKnowledgeSetup&&<button type="button" className="text-button" onClick={onKnowledgeSetup}>设置知识库</button>}</p>:<>
   <div className="ai-actions"><label><input type="radio" checked={mode==='create'} disabled={busy} onChange={()=>{setMode('create');setTarget(null);setPath(`${settings!.rootFolder||'映流'}/${safeNoteName(title)}`);}}/>新建笔记</label><label><input type="radio" checked={mode==='append'} disabled={busy} onChange={()=>{setMode('append');setTarget(null);setPath('');}}/>追加到已有笔记</label></div>
   {mode==='append'&&<div><label>查找已有笔记<input value={search} disabled={busy} onChange={e=>{setSearch(e.target.value);setResults(null);}} placeholder="输入笔记标题或内容"/></label><button type="button" className="secondary" disabled={busy||!search.trim()} onClick={()=>void findNotes()}>查找笔记</button>{results&&<div className="note-tool-list">{results.items.filter(hit=>hit.kind==='note'&&hit.path).map(hit=><button type="button" className="note-tool-entry" key={hit.path} disabled={busy} onClick={()=>void readTarget(hit.path!)}>{hit.title}<small>{hit.path}</small></button>)}{!results.items.some(hit=>hit.kind==='note')&&<p className="help">没有匹配的笔记</p>}{results.total>results.page*50&&<button type="button" className="text-button" disabled={busy} onClick={()=>void findNotes(results.page+1)}>下一页</button>}</div>}</div>}
   <label>{mode==='create'?'新笔记相对路径':'已有笔记相对路径'}<input value={path} disabled={busy} onChange={e=>{setPath(e.target.value);setTarget(null);}} placeholder="映流/笔记.md"/></label>
   {mode==='append'&&<button type="button" className="secondary" disabled={busy||!path.trim()} onClick={()=>void readTarget()}>读取目标笔记</button>}
   {target&&<p className="help">目标：{target.path} · 已读取原文，可检查完整写入结果</p>}
   <label>将写入的完整 Markdown<pre className="note-source-preview" style={{maxHeight:320,overflow:'auto',whiteSpace:'pre-wrap'}}>{proposed??'请先读取已有笔记'}</pre></label>
  </>}{error&&<p role="alert" className="error-text">{error}</p>}
  <footer><button type="button" className="secondary" disabled={busy} onClick={close}>关闭并丢弃草稿</button><button type="button" className="primary" disabled={busy||!vaultId||!path.trim()||!proposed} onClick={()=>void save()}>{busy?'正在保存…':'确认写入知识库'}</button></footer>
 </LearningDialog>;
}
