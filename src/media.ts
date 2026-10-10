import type {MediaForm,MediaInfo,MediaOperation,MediaRequest,OutputFormat} from './types/media.ts';
export function parseMediaTime(text:string):number {
 if(!/^\d{1,6}:\d{2}(?::\d{2})?(?:\.\d{1,3})?$/.test(text))throw new Error('请输入 MM:SS.mmm 或 HH:MM:SS.mmm 时间');
 const parts=text.split(':').map(Number);const seconds=parts.pop()!;const minutes=parts.pop()!;const hours=parts.pop()??0;
 if(seconds>=60||minutes>=60||hours>999999)throw new Error('分钟和秒必须小于 60');return hours*3600+minutes*60+seconds;
}
export function formatMediaTime(value:number|null|undefined):string {
 if(value==null||!Number.isFinite(value))return '未知';
 const ms=Math.round(Math.max(0,value)*1000);return `${String(Math.floor(ms/3600000)).padStart(2,'0')}:${String(Math.floor(ms/60000)%60).padStart(2,'0')}:${String(Math.floor(ms/1000)%60).padStart(2,'0')}.${String(ms%1000).padStart(3,'0')}`;
}
export function canUseOperation(info:MediaInfo,op:MediaOperation):boolean {
 if(op==='extractAudio')return info.audios.length>0;if(op==='remux')return !!(info.videos.length+info.audios.length);return info.videos.some(v=>!v.attachedPicture);
}
export function formatsFor(op:MediaOperation):OutputFormat[] {return op==='extractAudio'?['mp3','m4a','aac','wav','flac','opus']:op==='screenshot'?['png','jpg','webp']:['mp4','mkv','mov','webm'];}
export function createMediaRequest(info:MediaInfo,form:MediaForm,outputDir:string):MediaRequest {
 if(!canUseOperation(info,form.operation))throw new Error(form.operation==='extractAudio'?'文件没有音频轨道':'文件没有可处理的视频流');
 if(!formatsFor(form.operation).includes(form.outputFormat))throw new Error('目标格式与当前操作不匹配');
 if(form.outputFormat==='webm'&&form.operation!=='remux'&&!(form.operation==='trim'&&form.trimMode==='fast'))throw new Error('WebM 首版请使用极速无损转换或极速裁剪');
 const start=form.operation==='trim'||form.operation==='screenshot'?parseMediaTime(form.start??'00:00:00.000'):0;
 const end=form.operation==='trim'?parseMediaTime(form.end??''):null;
 if(end!=null&&(start>=end||info.duration==null||end>info.duration))throw new Error('裁剪时间必须满足：开始 < 结束 ≤ 媒体时长');
 if(form.operation==='screenshot'&&info.duration!=null&&start>=info.duration)throw new Error('截图时间超出视频时长');
 if(!outputDir)throw new Error('请选择保存目录');
 const videoCodec=form.videoCodec??'h264';
 return {inputPath:info.path,outputDir,operation:form.operation,outputFormat:form.outputFormat,videoCodec,audioCodec:form.audioCodec??'aac',quality:form.quality??'balanced',crf:form.advanced&&videoCodec!=='copy'?form.crf??23:null,height:form.advanced&&videoCodec!=='copy'?form.height||null:null,fps:form.advanced&&videoCodec!=='copy'?form.fps||null:null,audioBitrate:form.audioBitrate??320,start,end,trimMode:form.trimMode??'accurate',copyAudio:form.copyAudio??false};
}
export function isCurrentProbe(resultId:number,currentId:number){return resultId===currentId;}
export function mergeMediaTasks<T extends {id:string}>(tasks:T[],task:T):T[]{return tasks.some(t=>t.id===task.id)?tasks.map(t=>t.id===task.id?task:t):[...tasks,task];}
export function mediaBytes(n:number|null|undefined){if(n==null)return '未知';const units=['B','KB','MB','GB'];let u=0;while(n>=1024&&u<3){n/=1024;u++;}return `${n.toFixed(u?1:0)} ${units[u]}`;}
export function mediaBitrate(n:number|null|undefined){if(n==null)return '未知';return n>=1000000?`${(n/1000000).toFixed(1)} Mbps`:`${Math.round(n/1000)} kbps`;}
export function displayMediaPath(path:string){return path.replace(/^\\\\\?\\/,'');}
export function mediaParentDirectory(path:string){return path.slice(0,Math.max(path.lastIndexOf('/'),path.lastIndexOf('\\'))+1);}
