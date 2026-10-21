import {useTaskCenter} from '../tasks/TaskCenter';
import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {NotebookPen,ArrowUpRight} from 'lucide-react';
import type {VideoAnalysis} from '../../types/ai';
import type {KnowledgeReference,KnowledgeSettings} from '../../types/knowledge';
import {aiError} from './lib';

export function AnalysisNoteActions({analysis,onOpenKnowledge,onKnowledgeSetup}:{analysis:VideoAnalysis;onOpenKnowledge?:(reference:KnowledgeReference)=>void;onKnowledgeSetup?:()=>void}){
 const {trackOperation}=useTaskCenter();
 const [reference,setReference]=useState<KnowledgeReference|null>(null),[busy,setBusy]=useState(false),[message,setMessage]=useState(''),[missing,setMissing]=useState(false);
 const operation=useRef(0),locked=useRef(false);
 useEffect(()=>{++operation.current;setReference(null);setMessage('');setMissing(false);locked.current=false;setBusy(false);return()=>{++operation.current;};},[analysis.id]);
 useEffect(()=>{let alive=true;
  invoke<KnowledgeReference|null>('knowledge_analysis_note',{id:analysis.id}).then(ref=>{if(alive&&!locked.current)setReference(ref);}).catch(e=>{if(alive&&!locked.current)setMessage(aiError(e));});
  return()=>{alive=false;};
 },[analysis.id,analysis.markdownPath,analysis.markdownVaultId,analysis.updatedAt]);
 async function act(open:boolean){
  if(locked.current)return;locked.current=true;const ticket=++operation.current;setBusy(true);setMessage('');setMissing(false);
  try{
   const settings=await invoke<KnowledgeSettings>('knowledge_get_settings');if(ticket!==operation.current)return;
   if(!settings.vaultId){setMissing(true);setMessage('请先创建或选择知识库');return;}
   if(open){const latest=await invoke<KnowledgeReference|null>('knowledge_analysis_note',{id:analysis.id});if(ticket!==operation.current)return;if(!latest){setReference(null);setMessage('没有找到生成笔记，请重新生成');return;}const now=await invoke<KnowledgeSettings>('knowledge_get_settings');if(ticket!==operation.current)return;if(latest.vaultId!==settings.vaultId||now.vaultId!==settings.vaultId){setMessage('知识库已切换，请回到对应知识库后打开笔记');return;}setReference(latest);onOpenKnowledge?.(latest);}
   else{
    const result=await trackOperation('note','生成课程知识笔记',()=>invoke<KnowledgeReference&{warning:string|null}>('knowledge_generate_analysis_note',{id:analysis.id,vaultId:settings.vaultId}),{output:r=>r.path});
    if(ticket!==operation.current)return;
    const now=await invoke<KnowledgeSettings>('knowledge_get_settings');if(ticket!==operation.current)return;
    if(now.vaultId!==settings.vaultId||result.vaultId!==settings.vaultId){setMessage('知识库已切换，笔记已保留在原知识库');return;}
    setReference({vaultId:result.vaultId,path:result.path});setMessage(result.warning||'知识笔记已保存');
   }
  }catch(e){if(ticket===operation.current)setMessage(aiError(e));}finally{if(ticket===operation.current){locked.current=false;setBusy(false);}}
 }
 return <section className="course-knowledge-actions" aria-label="AI整理笔记" aria-busy={busy}><h4>AI整理笔记</h4><p className="help">将已有总结、转录和知识点保存为 Markdown，不会再次请求 AI。</p><div className="ai-actions"><button type="button" className="secondary" disabled={busy} onClick={()=>void act(false)}><NotebookPen size={15} aria-hidden="true"/>{busy?'正在处理笔记…':'生成知识笔记'}</button>{reference&&onOpenKnowledge&&<button type="button" className="text-button" disabled={busy} onClick={()=>void act(true)}><ArrowUpRight size={15} aria-hidden="true"/>打开知识笔记</button>}{missing&&onKnowledgeSetup&&<button type="button" className="text-button" onClick={onKnowledgeSetup}>设置知识库</button>}</div>{(message||analysis.noteWarning)&&<p className="help" role="status">{message||analysis.noteWarning}</p>}</section>;
}
