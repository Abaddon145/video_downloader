import {useRef} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Check,Loader2,X,FolderOpen,Play,RefreshCw} from 'lucide-react';
import {displayMediaPath,formatMediaTime} from '../../media';
import type {MediaTask} from '../../types/media';
import {ContextMenu,useContextMenu} from '../ContextMenu';
import type {ContextMenuItem} from '../ContextMenu';
import {batchInputPaths,canRemoveMediaRecords} from './mediaContextActions';
const labels={queued:'等待中',probing:'分析中',processing:'处理中',interrupted:'已中断',completed:'已完成',failed:'处理失败',cancelled:'已取消'};
const operations={remux:'无损封装',transcode:'格式转换',compress:'视频压缩',trim:'视频裁剪',extractAudio:'提取音频',screenshot:'视频截图',subtitleConvert:'字幕转换',subtitleMux:'外挂字幕',subtitleBurn:'字幕烧录',extractFrames:'视频抽帧',contactSheet:'联系表',coverExtract:'提取封面',coverSet:'设置封面',metadata:'元数据'};
export function MediaTaskRow({task,act,notify,checked,select,contextTasks,batch,remove}:{task:MediaTask;act:(command:string,args:Record<string,unknown>)=>Promise<void>;notify:(message:string)=>void;checked:boolean;select:(checked:boolean)=>void;contextTasks:MediaTask[];batch:(ids:string[])=>void;remove:(ids:string[])=>Promise<void>}){
 const {menu,open,close}=useContextMenu<{id:string;ids:string[]}>(),details=useRef<HTMLDetailsElement>(null);
 const busy=['queued','probing','processing'].includes(task.status);
 const retry=['failed','cancelled','interrupted'].includes(task.status);
 const name=task.inputPath.split(/[\\/]/).pop()||task.inputPath;
 const pathToCopy=task.outputPath??task.inputPath;
 const learningPath=[task.outputPath,task.inputPath].find(path=>path&&/\.(mp4|m4v|mov|mkv|avi|wmv|asf|webm|flv|mpg|mpeg|ts|m2ts|mts|3gp|3g2|ogv|mp3|aac|flac|wav|m4a|opus|ogg|wma)$/i.test(path));
 const items:ContextMenuItem[]=[
  {id:'openFile',label:'打开文件',disabled:task.status!=='completed'||!task.outputPath,group:'file'},
  {id:'openFolder',label:'打开所在文件夹',group:'file'},
  {id:'copyPath',label:'复制路径',group:'file'},
  {id:'addLearning',label:'加入学习库',disabled:!learningPath,group:'learning'},
  {id:'details',label:'查看详情',group:'learning'},
  {id:'cancel',label:'取消处理',disabled:!busy,danger:true,group:'task'},
  {id:'retry',label:'重新处理',disabled:!retry,group:'task'},
  {id:'batch',label:`批量整理${contextTasks.length>1?`（${contextTasks.length}项）`:''}`,disabled:batchInputPaths(contextTasks).length===0,group:'task'},
  {id:'remove',label:`安全移除记录${contextTasks.length>1?`（${contextTasks.length}项）`:''}`,disabled:!canRemoveMediaRecords(contextTasks),danger:true,group:'task'},
 ];
 const action=async(item:ContextMenuItem,target:{id:string;ids:string[]})=>{
  if(target.id!==task.id)throw new Error('媒体任务已变化，请重新打开菜单');
  const id=target.id;
  if(item.id==='openFile'||item.id==='openFolder'){await act('open_media_output',{id,folder:item.id==='openFolder'});return;}
  if(item.id==='copyPath'){await navigator.clipboard.writeText(pathToCopy);notify('路径已复制');return;}
  if(item.id==='addLearning'){if(!learningPath)throw new Error('没有可加入学习库的音视频文件');await invoke('learning_import_for_analysis',{path:learningPath});notify('已加入学习库');return;}
  if(item.id==='details'){if(details.current){details.current.open=true;}return;}
  if(item.id==='cancel'&&window.confirm('取消这个媒体任务？原文件会保留。')){await act('cancel_media_task',{id});return;}
  if(item.id==='retry')await act('retry_media_task',{id});
  if(item.id==='batch')batch(target.ids);
  if(item.id==='remove')await remove(target.ids);
 };
 return <article className="media-task-row" tabIndex={0} onContextMenu={event=>open(event,{id:task.id,ids:contextTasks.map(task=>task.id)})} onKeyDown={event=>open(event,{id:task.id,ids:contextTasks.map(task=>task.id)})}><div className="media-task-heading"><div><label className="check-label"><input type="checkbox" aria-label={`选择媒体任务：${name}`} checked={checked} onChange={event=>select(event.target.checked)}/></label><strong>{name}</strong><span>{operations[task.type]} · {task.request.outputFormat.toUpperCase()}</span></div><span className={task.status==='failed'?'error-text':'media-status'}>{task.status==='completed'?<Check size={15} aria-hidden="true"/>:busy?<Loader2 size={15} className="spin" aria-hidden="true"/>:null}{labels[task.status]}</span></div><p className="help">{task.phase}{task.request.batchId&&' · 批量任务'}{task.request.sourceDownloadId&&' · 下载完成自动处理'}</p>
 {busy&&<><progress aria-label={`${name}处理进度`} max={100} {...(task.progress==null?{}:{value:task.progress})}/><div className="media-progress-line"><span>{task.progress==null?'进度未知':`${task.progress.toFixed(1)}%`} · {formatMediaTime(task.processedTime)} / {formatMediaTime(task.totalDuration)}</span><span>{task.speed==null?'速度未知':`${task.speed.toFixed(1)}x`} · {task.eta==null?'剩余时间未知':`预计剩余 ${Math.ceil(task.eta)} 秒`}</span></div></>}
 {task.error&&<p className="error-text" role="alert">{task.error}</p>}{task.outputPath&&<p className="media-output-path" title={displayMediaPath(task.outputPath)}>{displayMediaPath(task.outputPath)}</p>}
 <div className="media-task-actions">{busy&&<button className="secondary" onClick={()=>{if(window.confirm('取消这个媒体任务？原文件会保留。'))void act('cancel_media_task',{id:task.id});}}><X size={15} aria-hidden="true"/>取消</button>}{retry&&<button className="secondary" onClick={()=>void act('retry_media_task',{id:task.id})}><RefreshCw size={15} aria-hidden="true"/>重新开始</button>}{task.status==='completed'&&<button className="secondary" onClick={()=>void act('open_media_output',{id:task.id,folder:false})}><Play size={15} aria-hidden="true"/>打开文件</button>}<button className="text-button" onClick={()=>void act('open_media_output',{id:task.id,folder:true})}><FolderOpen size={15} aria-hidden="true"/>打开目录</button></div><details ref={details} className="task-logs"><summary>任务详情（日志已脱敏）</summary><p>来源：{displayMediaPath(task.inputPath)}</p>{task.outputPath&&<p>输出：{displayMediaPath(task.outputPath)}</p>}<p>状态：{labels[task.status]} · {task.phase}</p><pre>{task.logs.join('\n')||'暂无日志'}</pre></details><ContextMenu menu={menu} items={items} onClose={close} onAction={action}/></article>;
}
