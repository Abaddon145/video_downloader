import {useTaskCenter} from '../tasks/TaskCenter';
import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Star,History,Files,Paperclip,Sparkles,Network} from 'lucide-react';
import {LearningDialog} from '../learning/Editors';
import {NoteMindMap} from './NoteMindMap';
import type {KnowledgeDocument,KnowledgeMutation,KnowledgeView} from '../../types/knowledge';
import type {AiSettingsView} from '../../types/ai';
import './noteTools.css';
export interface NoteSnapshot{vaultId:string;path:string;hash:string;text:string;editVersion:number}
interface Metadata{path:string;favorite:boolean;template:boolean;available:boolean}
interface Version{id:string;path:string;hash:string;bytes:number;createdAt:number;text?:string}
interface Attachment{path:string;name:string;hash:string;bytes:number;importedAt:number;state:string}
interface Page<T>{items:T[];total:number;page:number}
interface AiDraft{requestId:string;vaultId:string;path:string;sourceHash:string;inputHash:string;markdown:string}
type Kind='history'|'favorites'|'templates'|'attachments'|'ai'|'mindmap';
export type NoteToolRequest={id:number;path:string;kind:Kind|'favorite'};
const hash=async(text:string)=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text)))].map(b=>b.toString(16).padStart(2,'0')).join('');
export function NoteTools({vaultId,path,text,revision,prepare,apply,openNote,created,jump,request,onMove,onDelete,onExport}:{vaultId:string;path:string;text:string;revision:number;prepare:()=>Promise<NoteSnapshot>;apply:(text:string,snapshot:NoteSnapshot)=>boolean;openNote:(path:string)=>void;created:(path:string)=>void;jump:(id:string)=>void;request?:NoteToolRequest|null;onMove:()=>void;onDelete:()=>void;onExport:()=>void}){
 const {trackOperation}=useTaskCenter();
 const [metadata,setMetadata]=useState<Metadata|null>(null),[kind,setKind]=useState<Kind|null>(null),[snapshot,setSnapshot]=useState<NoteSnapshot|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 const [page,setPage]=useState(1),[list,setList]=useState<Page<Metadata|Version|Attachment>|null>(null),[preview,setPreview]=useState<KnowledgeDocument|Version|null>(null),[target,setTarget]=useState('');
 const [ai,setAi]=useState<AiSettingsView|null>(null),[consent,setConsent]=useState(false),[draft,setDraft]=useState<AiDraft|null>(null);
 const live=useRef(true),pendingRequest=useRef<string|null>(null),lastRequest=useRef(0),ticket=useRef(0);
 useEffect(()=>{live.current=true;return()=>{live.current=false;const id=pendingRequest.current;if(id)void invoke('ai_cancel_note_organize',{requestId:id}).catch(()=>{});};},[]);
 useEffect(()=>{let active=true;invoke<KnowledgeView<Metadata>>('knowledge_note_metadata',{vaultId,path}).then(r=>{if(active)setMetadata(r?.data??null);}).catch(e=>{if(active)setError(String(e));});return()=>{active=false;};},[vaultId,path,revision]);
 async function action(fn:()=>Promise<void>){setBusy(true);setError('');try{await fn();}catch(e){if(live.current)setError(String(e));}finally{if(live.current)setBusy(false);}}
 const close=()=>{++ticket.current;const id=pendingRequest.current;if(id)void invoke('ai_cancel_note_organize',{requestId:id}).catch(e=>setError(String(e)));setKind(null);setDraft(null);setPreview(null);};
 async function toggle(key:'favorite'|'template'){await action(async()=>{const m=metadata??(await invoke<KnowledgeView<Metadata>>('knowledge_note_metadata',{vaultId,path})).data;const r=await invoke<KnowledgeView<Metadata>>('knowledge_set_note_metadata',{vaultId,path,favorite:key==='favorite'?!m.favorite:m.favorite,template:key==='template'?!m.template:m.template});if(live.current)setMetadata(r.data);});}
 async function show(next:Kind){const own=++ticket.current;await action(async()=>{const s=await prepare();if(!live.current||ticket.current!==own)return;setSnapshot(s);setPage(1);setList(null);setPreview(null);setDraft(null);setConsent(false);setKind(next);if(next==='ai'){const settings=await invoke<AiSettingsView>('ai_get_settings');if(live.current&&ticket.current===own)setAi(settings);}});}
 useEffect(()=>{if(!request||request.id===lastRequest.current||request.path!==path)return;lastRequest.current=request.id;if(request.kind==='favorite')void toggle('favorite');else void show(request.kind);},[request?.id,path]);
 useEffect(()=>{if(!kind||kind==='ai'||kind==='mindmap')return;let active=true;setError('');setList(null);const command=kind==='history'?'knowledge_note_versions':kind==='attachments'?'knowledge_attachments':'knowledge_note_collection';void invoke<KnowledgeView<Page<Metadata|Version|Attachment>>>(command,{vaultId,...(kind==='history'?{path}:kind==='attachments'?{}:{kind}),page}).then(r=>{if(active)setList(r.data);}).catch(e=>{if(active)setError(String(e));});return()=>{active=false;};},[kind,page,vaultId,path,revision]);
 async function select(entry:Metadata|Version|Attachment){const own=++ticket.current;await action(async()=>{if(kind==='history'){const v=entry as Version;const r=await invoke<KnowledgeView<Version>>('knowledge_note_version',{vaultId,path,versionId:v.id});if(live.current&&ticket.current===own)setPreview(r.data);}else if(kind==='templates'){const r=await invoke<KnowledgeView<KnowledgeDocument>>('knowledge_read',{vaultId,path:entry.path});if(live.current&&ticket.current===own){setPreview(r.data);setTarget(entry.path.replace(/\.md$/i,' - 副本.md'));}}else if(kind==='favorites'){close();openNote(entry.path);}});}
 const applyPreview=()=>{if(snapshot&&preview?.text!==undefined){if(apply(preview.text,snapshot))close();else setError('笔记已发生变化，请关闭预览后重新操作；当前草稿已保留。');}};
 async function analyze(){if(!snapshot||!consent||!ai?.settings.allowNoteText)return;const s=snapshot,id=`note-${crypto.randomUUID()}`,own=++ticket.current;pendingRequest.current=id;
  await action(async()=>{try{const r=await trackOperation('note','AI 整理当前笔记',()=>invoke<AiDraft>('ai_organize_note',{request:{requestId:id,vaultId:s.vaultId,path:s.path,expectedHash:s.hash,text:s.text,allowNoteText:true}}));if(!live.current||ticket.current!==own)return;if(r.requestId!==id||r.vaultId!==s.vaultId||r.path!==s.path||r.sourceHash!==s.hash||r.inputHash!==await hash(s.text))throw Error('整理回执与当前笔记不一致，草稿未替换。');setDraft(r);}finally{if(pendingRequest.current===id)pendingRequest.current=null;}});
 }
 return <><div className="note-tools" role="toolbar" aria-label="知识笔记工具">
  <button className="text-button" disabled={busy} onClick={()=>{void show('ai');}}><Sparkles size={14}/>AI整理笔记</button>
  <details><summary style={{cursor:'pointer',padding:'7px'}}>更多</summary><div className="note-tools-more" style={{display:'flex',flexWrap:'wrap',gap:6,padding:6}}>
   <button className="text-button" disabled={busy} aria-pressed={metadata?.favorite??false} onClick={()=>{void toggle('favorite');}}><Star size={14}/>{metadata?.favorite?'已收藏':'收藏笔记'}</button>
   <button className="text-button" disabled={busy} aria-pressed={metadata?.template??false} onClick={()=>{void toggle('template');}}>{metadata?.template?'取消模板标记':'设为笔记模板'}</button>
   <button className="text-button" disabled={busy} onClick={()=>{void show('favorites');}}><Star size={14}/>收藏夹</button>
   <button className="text-button" disabled={busy} onClick={()=>{void show('history');}}><History size={14}/>版本历史</button>
   <button className="text-button" disabled={busy} onClick={()=>{void show('templates');}}><Files size={14}/>从模板新建</button>
   <button className="text-button" disabled={busy} onClick={()=>{void show('attachments');}}><Paperclip size={14}/>附件</button>
   <button className="text-button" disabled={busy} onClick={()=>{void show('mindmap');}}><Network size={14}/>思维导图</button>
   <button className="text-button" disabled={busy} onClick={onMove}>移动笔记</button>
   <button className="text-button" disabled={busy} onClick={onExport}>导出笔记</button>
   <button className="text-button" disabled={busy} onClick={onDelete}>删除笔记</button>
  </div></details>
 </div>{error&&!kind&&<p role="alert" className="error-text">{error}</p>}
 {kind==='mindmap'&&<NoteMindMap title={path.split('/').at(-1)??path} text={text} jump={jump} close={close}/>}
 {kind&&kind!=='mindmap'&&<LearningDialog title={{history:'版本历史',favorites:'收藏笔记',templates:'Markdown笔记模板',attachments:'知识库附件',ai:'AI整理笔记'}[kind]} close={close} busy={false}>
  {error&&<p role="alert" className="error-text">{error}</p>}
  {kind==='ai'?<><p className="help">只发送下方当前笔记文本给 DeepSeek，不发送视频、音频、附件或链接中的其他内容。整理可能产生 API 费用。结果会先预览，由你决定应用。</p><pre className="note-source-preview">{snapshot?.text}</pre>
   {!ai?.hasApiKey&&<p className="help">请先在设置的 Windows 安全窗口配置 API 密钥。</p>}
   {ai&&!ai.settings.allowNoteText&&<button className="secondary" disabled={busy} onClick={()=>{void action(async()=>{setAi(await invoke<AiSettingsView>('ai_save_settings',{settings:{...ai.settings,allowNoteText:true}}));});}}>启用笔记文本发送权限</button>}
   <label className="check-label"><input type="checkbox" checked={consent} onChange={e=>setConsent(e.target.checked)} disabled={busy}/>本次允许发送以上笔记文本</label>
   {draft&&<><h3>整理预览</h3><pre className="note-source-preview">{draft.markdown}</pre><button className="primary" disabled={busy} onClick={()=>{if(snapshot&&apply(draft.markdown,snapshot))close();else setError('笔记已改变，请重新整理；现有草稿已保留。');}}>应用到当前草稿</button></>}
   <footer><button className="secondary" onClick={close}>{busy?'取消整理':'关闭'}</button><button className="primary" disabled={busy||!consent||!ai?.hasApiKey||!ai.settings.allowNoteText||!snapshot?.text.trim()||new TextEncoder().encode(snapshot?.text??'').length>48*1024} onClick={()=>{void analyze();}}>{busy?'正在整理…':'确认并整理'}</button></footer>
  </>:<><div className="note-tool-list">{list?.items.map(entry=>kind==='attachments'?<article key={entry.path}><strong>{(entry as Attachment).name}</strong><small>{entry.path} · {Math.ceil((entry as Attachment).bytes/1024)} KiB · {(entry as Attachment).state}</small><div>
   <button className="secondary" disabled={busy||(entry as Attachment).state==='missing'} onClick={()=>{const a=entry as Attachment,s=snapshot;if(!s)return;const image=/\.(png|jpe?g|gif|webp)$/i.test(a.path),caption=a.name.replace(/[\[\]|\r\n]/g,' '),link=image?`![[${a.path}]]`:`[[${a.path}|${caption}]]`;if(apply(s.text+'\n\n'+link+'\n',s))close();else setError('笔记已改变，请重新选择插入位置。');}}>插入链接到笔记</button>
   <button className="secondary" disabled={busy||(entry as Attachment).state==='missing'} onClick={()=>{void action(async()=>{await invoke('knowledge_export_attachment',{vaultId,path:entry.path});});}}>导出附件</button>
   <button className="text-button" disabled={busy||(entry as Attachment).state==='missing'} onClick={()=>{if(window.confirm('将此附件移入回收目录？其他笔记中的引用可能失效。'))void action(async()=>{await invoke('knowledge_delete_attachment',{vaultId,path:entry.path,expectedHash:(entry as Attachment).hash});setKind(null);});}}>移入回收目录</button>
  </div></article>:<button className="note-tool-entry" key={'id'in entry?entry.id:entry.path} disabled={busy||'available'in entry&&!entry.available} onClick={()=>{void select(entry);}}>{kind==='history'?new Date((entry as Version).createdAt).toLocaleString():entry.path}<small>{kind==='history'?`${Math.ceil((entry as Version).bytes/1024)} KiB`:'available'in entry&&!entry.available?'文件已不存在':''}</small></button>)}</div>
   {list&&!list.items.length&&<p className="help">暂无记录。{kind==='history'?'修改并保存后会保留原文版本。':kind==='templates'?'先把一篇普通 Markdown 笔记标记为模板。':''}</p>}
   {list&&list.total>50&&<div className="note-tool-pagination"><button disabled={busy||page<=1} onClick={()=>setPage(n=>n-1)}>上一页</button><span>{page}/{Math.ceil(list.total/50)}</span><button disabled={busy||page*50>=list.total} onClick={()=>setPage(n=>n+1)}>下一页</button></div>}
   {preview&&<><pre className="note-source-preview">{preview.text}</pre>{kind==='history'?<button className="primary" disabled={busy} onClick={applyPreview}>恢复为当前草稿</button>:<><label>新笔记相对路径<input aria-label="模板新笔记路径" value={target} onChange={e=>setTarget(e.target.value)}/></label><button className="primary" disabled={busy||!target.trim()} onClick={()=>{void action(async()=>{const r=await invoke<KnowledgeMutation>('knowledge_create',{vaultId,path:target,text:preview.text,directory:false});if(live.current){close();created(r.path);}});}}>创建模板副本</button></>}</>}
   {kind==='attachments'&&<button className="primary" disabled={busy} onClick={()=>{void action(async()=>{const r=await invoke<KnowledgeView<Attachment>|null>('knowledge_import_attachment',{vaultId});if(r){setList(old=>({items:[r.data,...(old?.items??[])].slice(0,50),total:(old?.total??0)+1,page:1}));setPage(1);}});}}>导入附件</button>}
  </>}
 </LearningDialog>}
 </>;
}
