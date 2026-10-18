import {useEffect,useRef,useState,type ReactNode} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {Sparkles,FileText,NotebookPen,MessagesSquare} from 'lucide-react';
import type {AnalysisPage,AnalysisSource,VideoAnalysis} from '../../types/ai';
import type {KnowledgeReference,KnowledgeSettings} from '../../types/knowledge';
import {AnalysisNoteActions} from './AnalysisNoteActions';
import {AnalysisConfirm} from './AnalysisConfirm';
import {AnalysisProgress} from './AnalysisProgress';
import {VideoSummary} from './VideoSummary';
import {VideoAsk} from './VideoAsk';
import {Transcript} from './Transcript';
import {LearningDialog} from '../learning/Editors';
import {acceptCourseAnalysis,activeStatuses,aiError,modeLabels,statusLabels} from './lib';

type Tab='summary'|'transcript'|'questions'|'notes';
type DeletePlan={reference:KnowledgeReference;files:{path:string;hash:string}[];warning:string|null;analysisId:string};
export function PlayerAiSidebar({itemId,source,currentMs,onSeek,notes,requestVersion=0,requestAction='ai',onSettings,onOpenKnowledge,onKnowledgeSetup,notify}:{itemId:string;source:AnalysisSource;currentMs:number;onSeek:(ms:number)=>void;notes:ReactNode;requestVersion?:number;requestAction?:string;onSettings?:()=>void;onOpenKnowledge?:(reference:KnowledgeReference)=>void;onKnowledgeSetup?:()=>void;notify:(message:string)=>void}){
 const [analysis,setAnalysis]=useState<VideoAnalysis|null>(null),[tab,setTab]=useState<Tab>('summary');
 const [confirm,setConfirm]=useState(false),[force,setForce]=useState(false),[pending,setPending]=useState<'regenerate'|'delete'|null>(null);
 const [error,setError]=useState(''),[busy,setBusy]=useState(false),[loaded,setLoaded]=useState(false);
 const [history,setHistory]=useState<AnalysisPage|null>(null),[historyOpen,setHistoryOpen]=useState(false),[page,setPage]=useState(1),[refresh,setRefresh]=useState(0);
 const [jointDelete,setJointDelete]=useState(false),[deletePlan,setDeletePlan]=useState<DeletePlan|null>(null);
 const revision=useRef(0),requested=useRef(0),deleteOperation=useRef(0),deleteLocked=useRef(false);
 useEffect(()=>{++deleteOperation.current;deleteLocked.current=false;setDeletePlan(null);setJointDelete(false);setPending(p=>p==='delete'?null:p);setBusy(false);return()=>{++deleteOperation.current;};},[analysis?.id]);
 useEffect(()=>{let alive=true;const ticket=++revision.current;setAnalysis(null);setLoaded(false);setError('');setTab('summary');setHistory(null);setPage(1);
  invoke<VideoAnalysis|null>('ai_for_item',{itemId}).then(a=>{if(alive&&revision.current===ticket){setAnalysis(a);setLoaded(true);}}).catch(e=>{if(alive){setError(aiError(e));setLoaded(true);}});
  return()=>{alive=false;++revision.current;};},[itemId,source.path]);
 useEffect(()=>{let alive=true;const stops:(()=>void)[]=[];
  for(const event of ['ai-analysis-updated','ai-analysis-completed','ai-analysis-failed'])void listen<VideoAnalysis>(event,({payload:a})=>{
   if(!alive||a.itemId!==itemId||a.sourcePath.toLowerCase()!==source.path.toLowerCase())return;
   revision.current++;setLoaded(true);setAnalysis(current=>acceptCourseAnalysis(current,a,itemId));
   if(event!=='ai-analysis-updated')setRefresh(v=>v+1);
  }).then(stop=>alive?stops.push(stop):stop()).catch(e=>{if(alive)setError(aiError(e));});
  return()=>{alive=false;stops.forEach(stop=>stop());};},[itemId,source.path]);
 useEffect(()=>{if(requestVersion&&requestVersion!==requested.current&&loaded){requested.current=requestVersion;setTab('summary');
  if(requestAction==='ai-template'){setForce(!!analysis);setConfirm(true);}
  else if(requestAction==='ai-regenerate'&&analysis?.summary)setPending('regenerate');
  else if(requestAction==='ai-full'){setForce(true);setConfirm(true);}
  else if(!analysis||analysis.options.localOnly){setForce(false);setConfirm(true);}
 }},[requestVersion,requestAction,loaded,analysis]);
 useEffect(()=>{let alive=true;if(historyOpen)invoke<AnalysisPage>('ai_item_analyses',{itemId,page}).then(v=>{if(alive)setHistory(v);}).catch(e=>{if(alive)setError(aiError(e));});return()=>{alive=false;};},[itemId,source.path,page,historyOpen,refresh]);
 async function action(kind:'retry'|'regenerate'|'cancel'|'delete'){
  if(!analysis||deleteLocked.current)return;deleteLocked.current=true;const id=analysis.id,ticket=++deleteOperation.current;setBusy(true);setError('');
  try{
   if(kind==='delete'){const warning=await invoke<string|null>('ai_delete_analysis',{id});if(ticket!==deleteOperation.current)return;const current=await invoke<VideoAnalysis|null>('ai_for_item',{itemId});if(ticket!==deleteOperation.current)return;setAnalysis(current);notify(warning||'分析记录已删除，源视频和知识笔记已保留');}
   else if(kind==='cancel')await invoke('ai_cancel_analysis',{id});
   else{const a=await invoke<VideoAnalysis>('ai_retry_analysis',{id,regenerate:kind==='regenerate'});if(ticket!==deleteOperation.current)return;setAnalysis(current=>acceptCourseAnalysis(current,a,itemId));}
   if(ticket!==deleteOperation.current)return;
   setPending(null);setRefresh(v=>v+1);
  }catch(e){if(ticket===deleteOperation.current)setError(aiError(e));}finally{if(ticket===deleteOperation.current){deleteLocked.current=false;setBusy(false);}}
 }
 async function planDelete(){
  if(!analysis||busy||deleteLocked.current)return;deleteLocked.current=true;const id=analysis.id,ticket=++deleteOperation.current;setBusy(true);setError('');
  try{const settings=await invoke<KnowledgeSettings>('knowledge_get_settings');if(ticket!==deleteOperation.current)return;if(!settings.vaultId)throw '请先选择生成笔记所在的知识库';const plan=await invoke<Omit<DeletePlan,'analysisId'>>('knowledge_plan_analysis_delete',{id,vaultId:settings.vaultId});if(ticket!==deleteOperation.current)return;if(plan.reference.vaultId!==settings.vaultId)throw '知识库已切换，请重新确认';setDeletePlan({...plan,analysisId:id});}catch(e){if(ticket===deleteOperation.current)setError(aiError(e));}finally{if(ticket===deleteOperation.current){deleteLocked.current=false;setBusy(false);}}
 }
 async function deleteBundle(){
  if(!analysis||!deletePlan||busy||deleteLocked.current||analysis.id!==deletePlan.analysisId)return;deleteLocked.current=true;const ticket=++deleteOperation.current;setBusy(true);setError('');
  try{const result=await invoke<{recordDeleted:boolean;deletedPaths:string[];warning:string|null}>('knowledge_delete_analysis_bundle',{id:deletePlan.analysisId,vaultId:deletePlan.reference.vaultId,files:deletePlan.files});if(ticket!==deleteOperation.current)return;const current=await invoke<VideoAnalysis|null>('ai_for_item',{itemId});if(ticket!==deleteOperation.current)return;setAnalysis(current);setRefresh(v=>v+1);if(result.recordDeleted){setPending(null);setDeletePlan(null);notify(result.warning||'分析记录和确认的生成笔记已删除，源视频已保留');}else{setDeletePlan(null);setJointDelete(false);setError(result.warning||'部分笔记未能删除，分析记录已保留');}}catch(e){if(ticket===deleteOperation.current)setError(aiError(e));}finally{if(ticket===deleteOperation.current){deleteLocked.current=false;setBusy(false);}}
 }
 const running=!!analysis&&activeStatuses.includes(analysis.status);
 const tabs:[Tab,string,typeof Sparkles][]=[['summary','总结',Sparkles],['transcript','字幕',FileText],['questions','问答',MessagesSquare],['notes','笔记',NotebookPen]];
 return <aside className="learning-ai-sidebar" aria-label="课程学习面板">
  <header><div><span className="eyebrow">{analysis?.summary?'已整理这节课程':'围绕当前课程'}</span><h3>学习面板</h3></div><button type="button" className="secondary" disabled={busy||running} onClick={()=>{setForce(!!analysis?.summary);setConfirm(true);}}><Sparkles size={15} aria-hidden="true"/>{analysis?.summary?'重新完整分析':'AI分析'}</button></header>
  <div className="learning-ai-tabs" role="tablist" aria-label="课程侧栏" onKeyDown={e=>{const index=tabs.findIndex(([id])=>id===tab);if(!['ArrowLeft','ArrowRight','Home','End'].includes(e.key))return;e.preventDefault();const next=e.key==='Home'?0:e.key==='End'?tabs.length-1:(index+(e.key==='ArrowLeft'?-1:1)+tabs.length)%tabs.length;setTab(tabs[next][0]);e.currentTarget.querySelectorAll<HTMLButtonElement>('button')[next]?.focus();}}>{tabs.map(([id,label,Icon])=><button type="button" key={id} role="tab" id={`course-tab-${id}`} aria-selected={tab===id} aria-controls={`course-panel-${id}`} tabIndex={tab===id?0:-1} onClick={()=>setTab(id)}><Icon size={15} aria-hidden="true"/>{label}</button>)}</div>
  <div className="learning-ai-content">
   {error&&<p className="error-text" role="alert">{error}</p>}
   <div role="tabpanel" id="course-panel-summary" aria-labelledby="course-tab-summary" hidden={tab!=='summary'}>
    {!loaded&&<p role="status" className="help">正在读取课程分析…</p>}
    {analysis&&<>
     {(!analysis.summary||running)&&<AnalysisProgress analysis={analysis}/>}
     {analysis.error&&<p className="error-text" role="alert">{analysis.error}</p>}
     <div className="ai-actions">{running?<button type="button" className="secondary" disabled={busy} onClick={()=>void action('cancel')}>取消分析</button>:['failed','interrupted','cancelled'].includes(analysis.status)?<button type="button" className="secondary" disabled={busy} onClick={()=>void action('retry')}>继续分析</button>:null}</div>
     {analysis.summary?<VideoSummary result={analysis.summary} onSeek={onSeek}/>:!running&&<p className="help">{analysis.options.localOnly?'语音转录已保存在“字幕”页。开始分析可提取章节与知识点。':'该任务尚未生成总结，可以继续分析。'}</p>}
     {analysis.summary&&!running&&<button type="button" className="text-button" disabled={busy} onClick={()=>setPending('regenerate')}>重新生成总结</button>}
    </>}
    {loaded&&!analysis&&<div className="ai-empty"><Sparkles size={27} aria-hidden="true"/><h3>还没有生成视频总结</h3><p>在本地提取语音和关键画面，使用 DeepSeek 整理总结、章节与知识点。原视频不会上传。</p><button type="button" className="primary" onClick={()=>{setForce(false);setConfirm(true);}}>开始分析</button></div>}
    <details className="course-analysis-history" onToggle={e=>setHistoryOpen(e.currentTarget.open)}><summary>本课程分析记录</summary>{historyOpen&&history&&<>
     {!history.total&&<p className="help">这节课程还没有分析记录。</p>}
     {history.items.map(a=><button type="button" className="course-analysis-record" key={a.id} aria-pressed={analysis?.id===a.id} onClick={()=>setAnalysis(a)}><span>{modeLabels[a.analysisMode]} · {a.options.localOnly?'本地转录 · ':''}{statusLabels[a.status]??a.status}</span><small>{new Date(a.createdAt).toLocaleString('zh-CN')}</small></button>)}
     {history.total>50&&<div className="pagination"><button type="button" className="secondary" disabled={page<=1} onClick={()=>setPage(page-1)}>上一页记录</button><span>{history.page} / {Math.ceil(history.total/50)}</span><button type="button" className="secondary" disabled={page*50>=history.total} onClick={()=>setPage(page+1)}>下一页记录</button></div>}
    </>}{analysis&&!running&&<button type="button" className="text-button" disabled={busy} onClick={()=>{setPending('delete');setJointDelete(false);setDeletePlan(null);setError('');}}>删除当前分析记录</button>}</details>
   </div>
   <div role="tabpanel" id="course-panel-transcript" aria-labelledby="course-tab-transcript" hidden={tab!=='transcript'}>{tab==='transcript'&&<Transcript itemId={itemId} analysisId={analysis?.id} currentMs={currentMs} onSeek={onSeek} running={running} onRecognize={()=>{setForce(false);setConfirm(true);}}/>}</div>
   <div role="tabpanel" id="course-panel-questions" aria-labelledby="course-tab-questions" hidden={tab!=='questions'}>{analysis&&(analysis.transcriptDone||analysis.keyframesDone)?<VideoAsk key={analysis.id} analysisId={analysis.id} currentMs={currentMs} onSeek={onSeek}/>:<div className="ai-empty"><MessagesSquare size={26} aria-hidden="true"/><p>先分析当前课程，再查找相关片段或解释这一段。</p><button type="button" className="secondary" disabled={running} onClick={()=>{setForce(false);setConfirm(true);}}>开始分析</button></div>}</div>
   <div role="tabpanel" id="course-panel-notes" aria-labelledby="course-tab-notes" hidden={tab!=='notes'}>{notes}</div>
   {analysis?.summary&&!running&&<div hidden={tab!=='summary'&&tab!=='notes'}><AnalysisNoteActions key={analysis.id} analysis={analysis} onOpenKnowledge={onOpenKnowledge} onKnowledgeSetup={onKnowledgeSetup}/></div>}
  </div>
  {confirm&&<AnalysisConfirm source={{...source,force}} close={()=>setConfirm(false)} onSettings={()=>{setConfirm(false);if(onSettings)onSettings();else notify('请在主导航“设置”中配置AI服务');}} onStarted={a=>{setAnalysis(a);setLoaded(true);setTab('summary');setRefresh(v=>v+1);notify(a.status==='completed'?'已复用这节课程的分析':'课程已加入分析队列，完成后会显示在视频旁');}}/>}
  {pending&&<LearningDialog title={deletePlan?'确认删除生成笔记':pending==='delete'?'删除课程分析记录':'重新生成课程总结'} close={()=>setPending(null)} busy={busy}><p className="ai-confirm-message">{deletePlan?'请再次确认。仅删除下列生成文件及分析记录，源视频和其他用户文件会保留。':pending==='delete'?'删除本地转录、AI结果和该分析的临时缓存，保留源视频、Markdown知识笔记和原始用量统计。':'复用本地转录和画面，重新请求AI整理；这会产生新的API用量。'}</p>{pending==='delete'&&!deletePlan&&<label className="course-delete-notes"><input type="checkbox" checked={jointDelete} disabled={busy} onChange={e=>setJointDelete(e.target.checked)}/>同时删除生成的知识笔记</label>}{deletePlan&&<><ul className="course-delete-files">{deletePlan.files.map(file=><li key={file.path}>{file.path}</li>)}</ul>{deletePlan.warning&&<p className="help" role="status">{deletePlan.warning}</p>}</>}{error&&<p className="error-text" role="alert">{error}</p>}<footer><button type="button" className="secondary" disabled={busy} onClick={()=>setPending(null)}>取消</button><button type="button" className={deletePlan?'danger-button':'primary'} disabled={busy} onClick={()=>void(deletePlan?deleteBundle():pending==='delete'&&jointDelete?planDelete():action(pending))}>{deletePlan?'确认删除这些笔记和分析记录':pending==='delete'?(jointDelete?'预览将删除的笔记':'删除分析记录'):'确认重新生成'}</button></footer></LearningDialog>}
 </aside>;
}
