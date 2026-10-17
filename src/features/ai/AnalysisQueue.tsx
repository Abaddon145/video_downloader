
import {ChevronLeft,ChevronRight,FileVideo} from 'lucide-react';
import type {AnalysisPage} from '../../types/ai';
import {AnalysisCard} from './AnalysisCard';
export function AnalysisQueue({data,selected,select,page,setPage}:{data:AnalysisPage;selected:string|null;select:(id:string)=>void;page:number;setPage:(page:number)=>void}){
 const pages=Math.max(1,Math.ceil(data.total/50));return <section className="ai-queue" aria-label="分析任务"><div className="ai-section-heading"><h2>分析记录</h2><span>{data.total}</span></div>{data.items.length?data.items.map(a=><AnalysisCard key={a.id} analysis={a} selected={selected===a.id} select={()=>select(a.id)}/>):<div className="ai-empty"><FileVideo size={30}/><h3>从一个视频开始</h3><p>选择本地视频，提炼章节、知识点与学习步骤。</p></div>}{pages>1&&<div className="pagination"><button className="icon-button" aria-label="上一页分析" disabled={page<=1} onClick={()=>setPage(page-1)}><ChevronLeft size={18}/></button><span>{page} / {pages}</span><button className="icon-button" aria-label="下一页分析" disabled={page>=pages} onClick={()=>setPage(page+1)}><ChevronRight size={18}/></button></div>}</section>;
}
