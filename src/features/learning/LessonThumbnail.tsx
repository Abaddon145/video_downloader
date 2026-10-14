import {useEffect,useState} from 'react';
import {convertFileSrc} from '@tauri-apps/api/core';import {Check,Video} from 'lucide-react';
import type {LearningItem} from '../../types/learning';import {learningTime} from './lib';
export function LessonThumbnail({item,cover}:{item:LearningItem;cover:string|null}){
 const [index,setIndex]=useState(0);useEffect(()=>setIndex(0),[item.id,item.thumbnailPath,cover]);
 const sources=[item.thumbnailPath,cover].filter((p):p is string=>!!p&&!/^https?:/i.test(p));const source=sources[index];
 const percent=item.progress.completed?100:Math.max(0,Math.min(100,Math.round(item.progress.positionMs/(item.progress.durationMs||item.durationMs||1)*100)));
 return <span className="lesson-thumbnail">{source?<img src={convertFileSrc(source)} alt="" loading="lazy" decoding="async" width={128} height={72} onError={()=>setIndex(n=>n+1)}/>:<span className="lesson-thumbnail-placeholder" aria-hidden="true"><Video size={24} strokeWidth={1.4}/></span>}{item.progress.completed&&<span className="lesson-completed" aria-label="已完成"><Check size={13}/></span>}<span className="lesson-duration">{learningTime(item.durationMs)}</span><span className="lesson-watch-progress" role="progressbar" aria-label="课程观看进度" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}><span style={{width:`${percent}%`}}/></span></span>;
}
