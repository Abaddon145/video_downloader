export const seriesPercentage=(completed:number,total:number)=>total>0?Math.min(100,Math.max(0,Math.round(completed/total*100))):0;
export const resumePosition=(position:number,completed:boolean,restore:boolean)=>restore&&!completed&&position>=5000?position:0;
export function learningTime(ms:number|null|undefined){if(ms==null)return '时长未知';const t=Math.max(0,Math.floor(ms/1000));return t>=3600?`${Math.floor(t/3600)}:${String(Math.floor(t/60)%60).padStart(2,'0')}:${String(t%60).padStart(2,'0')}`:`${Math.floor(t/60)}:${String(t%60).padStart(2,'0')}`;}
export const learningError=(e:unknown)=>typeof e==='string'?e:e instanceof Error?e.message:'操作未完成，请重试';

import type {SeriesInput} from '../../types/learning';
export const seriesInput=({id,categoryId,title,description,coverPath,sourceUrl,favorite}:SeriesInput):SeriesInput=>({id,categoryId,title,description,coverPath,sourceUrl,favorite});
