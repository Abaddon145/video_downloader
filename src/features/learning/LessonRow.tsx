import {MoreHorizontal,Sparkles,Play,Download} from 'lucide-react';
import {ContextMenu,useContextMenu,type ContextMenuItem} from '../../components/ContextMenu';
import type {CourseAiState} from '../../types/ai';import {courseAnalysisLabel} from '../ai/lib';
import type {LearningItem} from '../../types/learning';import {LessonThumbnail} from './LessonThumbnail';import {SortHandle} from './Editors';
export type LessonAction='ai'|'ai-template'|'ai-retry'|'ai-regenerate'|'ai-full'|'note'|'play'|'mark'|'download'|'rename'|'relink'|'open'|'copy-path'|'move'|'manual'|'regenerate'|'remove';
export function LessonRow({item,cover,playing,downloadBusy,aiStatus,action,move}:{item:LearningItem;cover:string|null;playing:boolean;downloadBusy:boolean;aiStatus?:CourseAiState;action:(action:LessonAction)=>void;move:(change:Record<string,unknown>)=>void}){
 const {menu,open,close}=useContextMenu<LearningItem>();
 const status=item.available?(playing?'正在学习':item.progress.completed?'已完成':item.progress.positionMs>0?'继续学习':'开始学习'):({waiting:'尚未下载',queued:'等待下载',downloading:'下载中',paused:'已暂停',failed:'下载失败',cancelled:'已取消',missing:'文件缺失',completed:'文件待定位'}[item.downloadState]??'等待下载');
 const menuItems:ContextMenuItem[]=[
  {id:'play',label:item.progress.positionMs>0?'继续学习':'播放',disabled:!item.available,group:'play'},
  {id:'mark',label:item.progress.completed?'标记未完成':'标记完成',disabled:playing,group:'play'},
  {id:'download',label:'下载 / 重试下载',disabled:downloadBusy||!item.sourceUrl||['queued','downloading'].includes(item.downloadState),group:'play'},
  {id:'ai',label:'AI分析',disabled:!item.available,group:'ai'},
  {id:'ai-template',label:'选择AI模板分析',disabled:!item.available,group:'ai'},
  {id:'ai-retry',label:'继续失败的分析',disabled:!item.available||!aiStatus||!['failed','interrupted','cancelled'].includes(aiStatus.status),group:'ai'},
  {id:'ai-regenerate',label:'重新生成AI总结',disabled:!item.available||!aiStatus?.hasSummary,group:'ai'},
  {id:'ai-full',label:'重新完整分析',disabled:!item.available,group:'ai'},
  {id:'note',label:'打开关联笔记',disabled:!aiStatus?.hasSummary,group:'ai'},
  {id:'rename',label:'编辑课程名称',group:'edit'},
  {id:'manual',label:'设置缩略图',group:'edit'},
  {id:'regenerate',label:'重新生成封面',group:'edit'},
  {id:'relink',label:'重新定位视频',group:'edit'},
  {id:'move',label:'移动课程',group:'edit'},
  {id:'copy-path',label:'复制路径',disabled:!item.filePath,group:'file'},
  {id:'open',label:'打开文件位置',disabled:!item.available,group:'file'},
  {id:'remove',label:'移除课程',danger:true,group:'remove'}];
 return <article className={`learning-lesson${playing?' is-playing':''}`} tabIndex={0} onContextMenu={e=>open(e,item)} onKeyDown={e=>open(e,item)} data-sort-id={item.id} data-sort-scope="item" data-missing={!item.available} aria-current={playing?'true':undefined}>
  <button className="learning-lesson-play" aria-label={`播放 ${item.title}`} onClick={()=>action('play')}><LessonThumbnail item={item} cover={cover}/><span className="lesson-copy"><strong>{item.title}</strong><span className="lesson-state">{playing&&<Play size={12} fill="currentColor"/>}{status}</span></span></button>
  {!item.available&&item.sourceUrl&&!['queued','downloading'].includes(item.downloadState)&&<button className="icon-button lesson-download" aria-label={`${item.downloadState==='paused'?'继续下载':['failed','cancelled','missing'].includes(item.downloadState)?'重试下载':'下载课程'} ${item.title}`} disabled={downloadBusy} onClick={()=>action('download')}><Download size={18}/></button>}
  <button type="button" className="secondary lesson-ai-button" aria-label={`AI分析 ${item.title}`} disabled={!item.available} onClick={()=>action('ai')}><Sparkles size={15}/><span>{courseAnalysisLabel(aiStatus)}</span></button>
  <button className="icon-button" aria-haspopup="menu" aria-expanded={!!menu} aria-label={`课程操作 ${item.title}`} onClick={e=>open(e,item)}><MoreHorizontal size={20}/></button><SortHandle id={item.id} scope="item" move={move}/>
  <ContextMenu menu={menu} items={menuItems} onClose={close} onAction={entry=>action(entry.id as LessonAction)}/>
 </article>;
}
