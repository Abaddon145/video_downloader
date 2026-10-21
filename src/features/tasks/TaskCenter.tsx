import {createContext,useCallback,useContext,useEffect,useMemo,useRef,useState,type ReactNode} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {DownloadTask} from '../../types.ts';
import type {MediaSnapshot} from '../../types/media.ts';
import type {KnowledgeReference} from '../../types/knowledge';
import type {AnalysisPage,VideoAnalysis} from '../../types/ai.ts';
import {useFeedback} from '../../components/Feedback.tsx';
import {adaptAnalysis,adaptDownload,adaptMedia,adaptThumbnail,type ThumbnailTask,mergeInitial,taskPage,type CenterTask,type TaskAction,type TaskKind} from './lib.ts';
import './TaskCenter.css';

type OperationKind=Extract<TaskKind,'thumbnail'|'note'>;
type Callbacks<T>={output?:(value:T)=>string|null;onComplete?:(value:T)=>void;onError?:(error:unknown)=>void};
type ContextValue={tasks:CenterTask[];open:(filter?:string)=>void;trackOperation:<T>(kind:OperationKind,name:string,run:()=>Promise<T>,callbacks?:Callbacks<T>)=>Promise<T>};
const Context=createContext<ContextValue|null>(null);
export function useTaskCenter(){const value=useContext(Context);if(!value)throw new Error('TaskCenterProvider is missing');return value;}
const failText=(error:unknown)=>typeof error==='string'?error:error instanceof Error?error.message:'操作未完成';
const stateLabel={queued:'等待中',running:'运行中',paused:'已暂停',completed:'已完成',failed:'失败',cancelled:'已取消',needs_action:'需要操作'};
const kindLabel={download:'下载',media:'媒体处理',ai:'AI 分析',thumbnail:'缩略图',note:'笔记'};
const isBusy=(task:CenterTask)=>task.state==='queued'||task.state==='running';

export function TaskCenterProvider({downloads,children,onOpenVideo,onOpenKnowledge}:{downloads:DownloadTask[];children:ReactNode;onOpenVideo?:(itemId:string,timeMs:number,action:'ai')=>void;onOpenKnowledge?:(reference:KnowledgeReference)=>void}){
 const [media,setMedia]=useState<MediaSnapshot['tasks']>([]),[analyses,setAnalyses]=useState<VideoAnalysis[]>([]),[operations,setOperations]=useState<CenterTask[]>([]);
 const [open,setOpen]=useState(false),[selected,setSelected]=useState<string|null>(null),[loadError,setLoadError]=useState(''),[reload,setReload]=useState(0),[aiPage,setAiPage]=useState(0),[aiTotal,setAiTotal]=useState(0),[aiLoading,setAiLoading]=useState(false);
 const count=useRef(0),previous=useRef(new Map<string,CenterTask>()),drawer=useRef<HTMLElement>(null),modal=useRef<HTMLDialogElement>(null),aiEvents=useRef(new Map<string,VideoAnalysis>());
 const {notify}=useFeedback();
 const tasks=useMemo(()=>[...downloads.map(adaptDownload),...media.map(adaptMedia),...analyses.map(adaptAnalysis),...operations].sort((a,b)=>b.createdAt-a.createdAt),[downloads,media,analyses,operations]);
 useEffect(()=>{
  const next=new Map(tasks.map(task=>[task.id,task]));
  for(const task of tasks){const old=previous.current.get(task.id);if(old&&old.state!==task.state&&(task.state==='completed'||task.state==='failed'))notify(`${kindLabel[task.kind]}${task.state==='completed'?'已完成':'失败'}`,{label:'查看任务',run:()=>{setSelected(task.id);setOpen(true);}});}
  previous.current=next;
 },[tasks,notify]);
 useEffect(()=>{
  if(!isTauri())return;
  let alive=true,mediaEvent=false;const thumbnailEvents=new Map<string,CenterTask>();
  const stops:(()=>void)[]=[];aiEvents.current=new Map();setLoadError('');
  const setup=async()=>{
   const mediaStop=await listen<MediaSnapshot>('media-snapshot-updated',({payload})=>{if(alive){mediaEvent=true;setMedia(payload.tasks);}});
   if(!alive){mediaStop();return;}stops.push(mediaStop);
   for(const event of ['ai-analysis-updated','ai-analysis-completed','ai-analysis-failed']){
    const stop=await listen<VideoAnalysis>(event,({payload})=>{if(!alive)return;const old=aiEvents.current.get(payload.id);if(!old||payload.updatedAt>=old.updatedAt){aiEvents.current.set(payload.id,payload);if(aiEvents.current.size>100)aiEvents.current.delete(aiEvents.current.keys().next().value!);setAnalyses(current=>mergeInitial(current,[payload],'ai').sort((a,b)=>b.createdAt-a.createdAt).slice(0,500));}});
    if(!alive){stop();return;}stops.push(stop);
   }
   const thumbnailStop=await listen<ThumbnailTask>('learning-thumbnail-task',({payload})=>{if(!alive)return;const task=adaptThumbnail(payload);thumbnailEvents.set(task.id,task);setOperations(current=>[task,...current.filter(t=>t.id!==task.id)].slice(0,100));});
   if(!alive){thumbnailStop();return;}stops.push(thumbnailStop);
   const [thumbnails,mediaState,result]=await Promise.allSettled([invoke<ThumbnailTask[]>('learning_thumbnail_tasks'),invoke<MediaSnapshot>('get_media_snapshot'),invoke<AnalysisPage>('ai_list_analyses',{status:null,page:1})]);
   if(!alive)return;
   if(thumbnails.status==='fulfilled')setOperations(current=>{const merged=new Map<string,CenterTask>();for(const task of [...thumbnails.value.map(adaptThumbnail),...current,...thumbnailEvents.values()]){const previous=merged.get(task.id);if(!previous||(task.finishedAt??task.createdAt)>=(previous.finishedAt??previous.createdAt))merged.set(task.id,task);}return [...merged.values()].sort((a,b)=>b.createdAt-a.createdAt).slice(0,100);});
   if(mediaState.status==='fulfilled'&&!mediaEvent)setMedia(mediaState.value.tasks);
   if(result.status==='fulfilled'){setAnalyses(mergeInitial(result.value.items,[...aiEvents.current.values()],'ai'));setAiPage(1);setAiTotal(result.value.total);}
   if([thumbnails,mediaState,result].some(value=>value.status==='rejected'))setLoadError('部分任务状态暂时无法读取，其他任务仍可使用');
  };
  void setup().catch(()=>{if(alive)setLoadError('任务状态暂时无法读取');});
  return()=>{alive=false;stops.forEach(stop=>stop());};
 },[reload]);
 const loadMore=async()=>{if(aiLoading||aiPage===0||aiPage*50>=aiTotal||analyses.length>=500)return;setAiLoading(true);setLoadError('');try{const result=await invoke<AnalysisPage>('ai_list_analyses',{status:null,page:aiPage+1});setAnalyses(current=>mergeInitial([...current,...result.items],[...aiEvents.current.values()],'ai').sort((a,b)=>b.createdAt-a.createdAt).slice(0,500));setAiPage(result.page);setAiTotal(result.total);}catch{setLoadError('较早的 AI 任务暂时无法读取');}finally{setAiLoading(false);}};
 const trackOperation=useCallback(async<T,>(kind:OperationKind,name:string,run:()=>Promise<T>,callbacks?:Callbacks<T>):Promise<T>=>{
  const id=`${kind}:session-${Date.now()}-${++count.current}`,createdAt=Date.now();
  const base:CenterTask={id,sourceId:id,kind,name,state:'running',phase:'本次会话中运行；重启后不恢复',progress:null,createdAt,finishedAt:null,output:null,error:null,logs:[],actions:[],transient:true};
  setOperations(current=>[base,...current].slice(0,100));
  try{const result=await run();setOperations(current=>current.map(item=>item.id===id?{...item,state:'completed',phase:'已完成',output:callbacks?.output?.(result)??null,finishedAt:Date.now()}:item));callbacks?.onComplete?.(result);return result;}
  catch(error){setOperations(current=>current.map(item=>item.id===id?{...item,state:'failed',phase:'操作未完成',error:failText(error),finishedAt:Date.now()}:item));callbacks?.onError?.(error);throw error;}
 },[]);
 const value=useMemo<ContextValue>(()=>({tasks,open:(filter='all')=>{setFilter(filter);setPage(1);setOpen(true);},trackOperation}),[tasks,trackOperation]);
 useEffect(()=>{if(!open)return;const previousFocus=document.activeElement as HTMLElement|null;const dialog=modal.current;dialog?.showModal();drawer.current?.focus();return()=>{dialog?.close();if(previousFocus?.isConnected)previousFocus.focus();};},[open]);
 const [filter,setFilter]=useState('all'),[page,setPage]=useState(1),[busy,setBusy]=useState<string|null>(null),[actionError,setActionError]=useState(''),[now,setNow]=useState(Date.now());
 useEffect(()=>{if(!open)return;const timer=setInterval(()=>setNow(Date.now()),1000);return()=>clearInterval(timer);},[open]);
 const filtered=tasks.filter(task=>filter==='all'||filter===task.kind||filter===task.state||filter==='transcription'&&task.typeLabel==='本地字幕');
 const listed=taskPage(filtered,page),detail=tasks.find(task=>task.id===selected);
 const act=async(task:CenterTask,action:TaskAction|'folder')=>{
  setBusy(task.id);setActionError('');
  try{
   if(!task.actions.includes(action as TaskAction)&&action!=='folder')throw Error('当前任务不支持此操作');
   if(task.kind==='download')await invoke(action==='open'||action==='folder'?'open_task_target':'control_task',action==='open'||action==='folder'?{id:task.sourceId,folder:action==='folder'}:{id:task.sourceId,action});
   else if(task.kind==='media')await invoke(action==='open'||action==='folder'?'open_media_output':action==='retry'?'retry_media_task':'cancel_media_task',action==='open'||action==='folder'?{id:task.sourceId,folder:action==='folder'}:{id:task.sourceId});
   else if(task.kind==='thumbnail'){if(!task.itemId)throw Error('课程记录不可用');await invoke('learning_regenerate_thumbnail',{id:task.itemId,confirmed:false});}
   else if(task.kind==='ai'&&action==='open'){const reference=await invoke<KnowledgeReference|null>('knowledge_analysis_note',{id:task.sourceId});if(!reference)throw Error('尚未找到生成的知识笔记');if(onOpenKnowledge){setOpen(false);onOpenKnowledge(reference);}else throw Error('知识库入口尚未连接');}
   else if(task.kind==='ai')await invoke(action==='retry'?'ai_retry_analysis':'ai_cancel_analysis',action==='retry'?{id:task.sourceId,regenerate:false}:{id:task.sourceId});
  }catch(error){setActionError(failText(error));}finally{setBusy(null);}
 };
 return <Context.Provider value={value}>{children}
  {open&&<dialog ref={modal} className="task-center-modal" aria-label="后台任务" onCancel={event=>{event.preventDefault();setOpen(false);}} onMouseDown={event=>{const box=event.currentTarget.getBoundingClientRect();if(event.clientX<box.left||event.clientX>box.right||event.clientY<box.top||event.clientY>box.bottom)setOpen(false);}}><section ref={drawer} tabIndex={-1} className="task-center-drawer" onKeyDown={event=>{if(event.key==='Tab'){const controls=[...event.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),select:not(:disabled),summary,[tabindex="0"]')];const first=controls[0],last=controls.at(-1);if(document.activeElement===event.currentTarget){event.preventDefault();(event.shiftKey?last:first)?.focus();}else if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus();}}}}>
   <header><h2>后台任务 <small>{tasks.length}</small></h2><button type="button" onClick={()=>setOpen(false)} aria-label="关闭任务中心">关闭</button></header>
   <div className="task-center-filters"><label>筛选 <select value={filter} onChange={event=>{setFilter(event.target.value);setPage(1);setSelected(null);}}><option value="all">全部</option><option value="download">下载</option><option value="media">媒体</option><option value="ai">AI</option><option value="transcription">本地字幕</option><option value="thumbnail">缩略图</option><option value="note">笔记</option><option value="queued">等待中</option><option value="running">运行中</option><option value="completed">已完成</option><option value="failed">失败</option><option value="needs_action">需要操作</option></select></label></div>
   {actionError&&<p role="alert" className="error-text">{actionError}</p>}
   {loadError&&<p role="alert" className="error-text">{loadError} <button type="button" onClick={()=>setReload(value=>value+1)}>重试读取</button></p>}
   {listed.items.length?<div className="task-center-list">{listed.items.map(task=><article key={task.id} className="task-center-item"><button className="task-center-item-title" onClick={()=>setSelected(task.id)} aria-expanded={selected===task.id}><strong>{task.name}</strong><span>{task.typeLabel??kindLabel[task.kind]} · {stateLabel[task.state]}</span></button><p>{task.phase}</p>{isBusy(task)&&<progress max={100} {...(task.progress===null?{}:{value:task.progress})} aria-label={`${task.name}进度`}/>}<div className="task-center-actions">{task.actions.map(action=><button key={action} type="button" disabled={busy===task.id} onClick={()=>void act(task,action)}>{({pause:'暂停',resume:'继续',cancel:'取消',retry:'重试',open:'打开结果'} as const)[action]}</button>)}{(task.kind==='download'||task.kind==='media')&&<button type="button" disabled={busy===task.id} onClick={()=>void act(task,'folder')}>打开目录</button>}{task.kind==='ai'&&task.itemId&&onOpenVideo&&<button type="button" onClick={()=>{setOpen(false);onOpenVideo(task.itemId!,0,'ai');}}>打开课程</button>}</div></article>)}</div>:<p className="task-center-empty">暂无符合条件的任务</p>}
   {listed.pages>1&&<nav aria-label="任务分页"><button disabled={listed.page===1} onClick={()=>setPage(listed.page-1)}>上一页</button><span>{listed.page} / {listed.pages}</span><button disabled={listed.page===listed.pages} onClick={()=>setPage(listed.page+1)}>下一页</button></nav>}
   {aiPage>0&&aiPage*50<aiTotal&&analyses.length<500&&<button type="button" className="task-center-more" disabled={aiLoading} onClick={()=>void loadMore()}>{aiLoading?'正在读取…':'加载更早的 AI 任务'}</button>}
   {analyses.length>=500&&<p className="help">任务中心显示最近 500 条 AI 记录，更早的记录仍保留在分析历史中。</p>}
   {detail&&<aside className="task-center-detail"><h3>任务详情</h3>{detail.transient&&<p>本次会话操作，重启后不恢复记录。</p>}<dl><dt>类型</dt><dd>{kindLabel[detail.kind]}</dd><dt>状态</dt><dd>{stateLabel[detail.state]}</dd><dt>阶段</dt><dd>{detail.phase}</dd><dt>创建时间</dt><dd>{new Date(detail.createdAt).toLocaleString()}</dd><dt>记录时长（含等待）</dt><dd>{Math.max(0,Math.round(((detail.finishedAt??now)-detail.createdAt)/1000))} 秒</dd>{detail.output&&<><dt>输出</dt><dd>{detail.output}</dd></>}{detail.error&&<><dt>错误</dt><dd>{detail.error}</dd></>}</dl>{detail.logs.length>0&&<details><summary>日志</summary><pre>{detail.logs.join('\n')}</pre></details>}</aside>}
  </section></dialog>}
 </Context.Provider>;
}

export function TaskCenterButton(){const {tasks,open}=useTaskCenter();const busy=tasks.filter(isBusy).length;return <button type="button" className="secondary" onClick={()=>open()} aria-label={`后台任务，${busy} 项进行中`}>后台任务{busy?` · ${busy}`:''}</button>;}
