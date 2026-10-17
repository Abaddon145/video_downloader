
export const modeLabels={quick:'快速',standard:'标准',deep:'深度'};
export const activeStatuses=['queued','extracting_audio','transcribing','extracting_keyframes','analyzing_chunks','finalizing','writing_note'];
export const statusLabels:Record<string,string>={queued:'等待分析',extracting_audio:'读取音频',transcribing:'识别语音',extracting_keyframes:'分析画面',analyzing_chunks:'理解内容',finalizing:'整理总结',writing_note:'生成笔记',completed:'已完成',failed:'分析失败',cancelled:'已取消',interrupted:'分析被中断'};
export function analysisActions(status:string,localOnly=false):string[]{return activeStatuses.includes(status)?['cancel']:status==='completed'?(localOnly?['reanalyze','delete']:['regenerate','reanalyze','delete']):['retry','reanalyze','delete'];}
export function acceptAnalysis<T extends {id:string;updatedAt:number}>(current:T,incoming:T):T{return current.id===incoming.id&&incoming.updatedAt>=current.updatedAt?incoming:current;}
export function mediaTitle(path:string){return path.split(/[\\/]/).at(-1)?.replace(/\.[^.]+$/,'')||'本地媒体';}
export function aiError(error:unknown){return typeof error==='string'?error:error instanceof Error?error.message:'操作未完成，请重试';}
export function fileSize(bytes:number){return bytes>=1024**3?`${(bytes/1024**3).toFixed(1)} GB`:bytes>=1024**2?`${(bytes/1024**2).toFixed(1)} MB`:`${Math.ceil(bytes/1024)} KB`;}
export function timeLabel(ms:number){const s=Math.floor(Math.max(0,ms)/1000);return s>=3600?`${Math.floor(s/3600)}:${String(Math.floor(s/60)%60).padStart(2,'0')}:${String(s%60).padStart(2,'0')}`:`${Math.floor(s/60).toString().padStart(2,'0')}:${String(s%60).padStart(2,'0')}`;}

export function acceptCourseAnalysis<T extends {id:string;itemId:string|null;createdAt:number;updatedAt:number;options:{localOnly:boolean}}>(current:T|null,incoming:T,itemId:string):T|null{
 if(incoming.itemId!==itemId)return current;
 if(!current)return incoming;
 if(!current.options.localOnly&&incoming.options.localOnly)return current;
 if(current.id===incoming.id)return acceptAnalysis(current,incoming);
 return incoming.createdAt>=current.createdAt?incoming:current;
}

export function courseAnalysisLabel(value?:{status:string;progress:number}){
 if(!value||value.status==='cancelled')return '未分析';
 if(value.status==='queued')return '排队中';
 if(activeStatuses.includes(value.status))return `分析中 ${Math.round(Math.min(1,Math.max(0,value.progress||0))*100)}%`;
 return ({completed:'已分析',failed:'分析失败',interrupted:'分析中断'} as Record<string,string>)[value.status]??'未分析';
}
