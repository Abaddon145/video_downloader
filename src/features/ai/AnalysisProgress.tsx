
import type {VideoAnalysis} from '../../types/ai';
import {activeStatuses,statusLabels} from './lib';
const steps=[['extracting_audio','读取媒体'],['transcribing','本地识别语音'],['extracting_keyframes','分析画面'],['analyzing_chunks','理解内容'],['finalizing','整理知识']];
export function AnalysisProgress({analysis}:{analysis:VideoAnalysis}){
 const active=activeStatuses.includes(analysis.status),index=steps.findIndex(([id])=>id===analysis.status);
 return <div className="ai-progress"><div><strong>{analysis.stage||statusLabels[analysis.status]}</strong><span>{Math.round(analysis.progress*100)}%</span></div><progress aria-label="分析进度" max={1} value={analysis.progress}/>{active&&<ol>{steps.map(([id,label],i)=><li key={id} aria-current={i===index?'step':undefined} className={i<index?'done':i===index?'current':''}>{label}</li>)}</ol>}</div>;
}
