
import {FileVideo,ChevronRight} from 'lucide-react';
import type {VideoAnalysis} from '../../types/ai';
import {mediaTitle,modeLabels,statusLabels,activeStatuses,timeLabel} from './lib';
export function AnalysisCard({analysis,selected,select}:{analysis:VideoAnalysis;selected:boolean;select:()=>void}){
 return <button className={`ai-analysis-card ${selected?'selected':''}`} onClick={select} aria-pressed={selected}><span className="ai-media-icon"><FileVideo size={22}/></span><span className="ai-card-copy"><strong title={analysis.sourcePath}>{(analysis.title||mediaTitle(analysis.sourcePath))}</strong><small>{analysis.options.localOnly?'本地转录':`${modeLabels[analysis.analysisMode]}分析`} · {analysis.durationMs?timeLabel(analysis.durationMs):'读取后显示时长'}</small><span className={`ai-state ${analysis.status}`}>{statusLabels[analysis.status]||analysis.status}{activeStatuses.includes(analysis.status)&&` · ${Math.round(analysis.progress*100)}%`}</span></span><ChevronRight size={15}/></button>;
}
