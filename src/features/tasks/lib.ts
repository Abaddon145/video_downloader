import type {DownloadTask} from '../../types.ts';
import type {MediaTask} from '../../types/media.ts';
import type {VideoAnalysis} from '../../types/ai.ts';
import {taskActionAllowed} from '../../lib.ts';
import {activeStatuses,mediaTitle,statusLabels} from '../ai/lib.ts';

export type TaskKind='download'|'media'|'ai'|'thumbnail'|'note';
export type TaskState='queued'|'running'|'paused'|'completed'|'failed'|'cancelled'|'needs_action';
export type TaskAction='pause'|'resume'|'cancel'|'retry'|'open';
export interface CenterTask {
 id:string;sourceId:string;kind:TaskKind;name:string;state:TaskState;phase:string;progress:number|null;
 createdAt:number;finishedAt:number|null;output:string|null;error:string|null;logs:string[];actions:TaskAction[];
 itemId?:string|null;transient?:boolean;typeLabel?:string;
}
export interface ThumbnailTask{id:string;itemId:string;name:string;status:'running'|'completed'|'failed'|'needs_action';phase:string;createdAt:number;finishedAt:number|null;output:string|null;error:string|null}
export function adaptThumbnail(task:ThumbnailTask):CenterTask{return {id:`thumbnail:${task.id}`,sourceId:task.id,itemId:task.itemId,kind:'thumbnail',name:task.name,state:task.status,phase:task.status==='running'?'正在生成课程缩略图':task.status==='completed'?'课程缩略图已生成':'没有生成可用画面，请检查来源后重试',progress:null,createdAt:task.createdAt,finishedAt:task.finishedAt,output:task.output,error:task.error,logs:[],actions:task.status==='failed'||task.status==='needs_action'?['retry']:[],transient:true};}

const fileName=(path:string)=>path.split(/[\\/]/).at(-1)||path;
const percent=(value:number|null|undefined)=>value!=null&&Number.isFinite(value)?Math.max(0,Math.min(100,value)):null;
export function adaptDownload(task:DownloadTask):CenterTask {
 const state:TaskState=task.status==='queued'?'queued':task.status==='paused'?'paused':task.status==='completed'?'completed':task.status==='failed'?'failed':task.status==='cancelled'?'cancelled':'running';
 return {id:`download:${task.id}`,sourceId:task.id,kind:'download',name:task.request.title||'下载任务',state,phase:task.phase||'下载',progress:task.status==='downloading'?percent(task.progress.percent):task.status==='completed'?100:null,createdAt:task.createdAt,finishedAt:task.finishedAt,output:task.files[0]??task.outputDir??null,error:task.error,logs:task.logs??[],actions:([
  taskActionAllowed(task.status,'pause')&&'pause',taskActionAllowed(task.status,'resume')&&'resume',taskActionAllowed(task.status,'cancel')&&'cancel',taskActionAllowed(task.status,'retry')&&'retry',task.status==='completed'&&task.files.length>0&&'open'
 ] as (TaskAction|false)[]).filter((value):value is TaskAction=>!!value)};
}
export function adaptMedia(task:MediaTask):CenterTask {
 const state:TaskState=task.status==='queued'?'queued':task.status==='interrupted'?'needs_action':task.status==='completed'?'completed':task.status==='failed'?'failed':task.status==='cancelled'?'cancelled':'running';
 return {id:`media:${task.id}`,sourceId:task.id,kind:'media',name:fileName(task.inputPath),state,phase:task.phase||task.type,progress:percent(task.progress),createdAt:task.createdAt,finishedAt:task.finishedAt,output:task.outputPath??task.request?.outputDir??null,error:task.error,logs:task.logs??[],actions:([
  ['queued','probing','processing'].includes(task.status)&&'cancel',['failed','cancelled','interrupted'].includes(task.status)&&'retry',task.status==='completed'&&!!task.outputPath&&'open'
 ] as (TaskAction|false)[]).filter((value):value is TaskAction=>!!value)};
}
export function adaptAnalysis(task:VideoAnalysis):CenterTask {
 const state:TaskState=task.status==='queued'?'queued':task.status==='interrupted'?'needs_action':task.status==='completed'?'completed':task.status==='failed'?'failed':task.status==='cancelled'?'cancelled':'running';
 const active=activeStatuses.includes(task.status);
 return {id:`ai:${task.id}`,sourceId:task.id,kind:'ai',typeLabel:task.options.localOnly?'本地字幕':'AI 分析',name:task.title||mediaTitle(task.sourcePath),state,phase:task.stage||statusLabels[task.status]||'AI 分析',progress:state==='completed'?100:null,createdAt:task.createdAt,finishedAt:task.completedAt,output:task.markdownPath??null,error:task.error,logs:[],actions:active?['cancel']:['failed','cancelled','interrupted'].includes(task.status)?['retry']:task.status==='completed'&&task.markdownPath?['open']:[],itemId:task.itemId};
}

// Events observed after a listener starts override the in-flight bootstrap snapshot.
export function mergeInitial<T extends {id:string;updatedAt?:number}>(snapshot:T[],events:T[],kind:'ai'|'media'):T[]{
 const merged=new Map<string,T>();
 for(const item of snapshot){const old=merged.get(item.id);if(!old||kind==='media'||(item.updatedAt??0)>=(old.updatedAt??0))merged.set(item.id,item);}
 for(const event of events){const old=merged.get(event.id);if(!old||kind==='media'||(event.updatedAt??0)>=(old.updatedAt??0))merged.set(event.id,event);}
 return [...merged.values()];
}
export function taskPage<T>(items:T[],requested:number){const pages=Math.max(1,Math.ceil(items.length/50)),page=Math.max(1,Math.min(requested,pages));return {items:items.slice((page-1)*50,page*50),page,pages};}
