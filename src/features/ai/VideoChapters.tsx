
import type {Chapter} from '../../types/ai';
import {timeLabel} from './lib';
export function TimeButton({ms,onSeek}:{ms:number;onSeek?:(ms:number)=>void}){return onSeek?<button className="ai-time" onClick={()=>onSeek(ms)}>{timeLabel(ms)}</button>:<span className="ai-time">{timeLabel(ms)}</span>;}
export function VideoChapters({chapters,onSeek}:{chapters:Chapter[];onSeek?:(ms:number)=>void}){return <section className="ai-result-section"><h3>自动章节</h3>{chapters.length?<ol className="ai-chapters">{chapters.map((c,i)=><li key={`${c.startMs}-${i}`}><TimeButton ms={c.startMs} onSeek={onSeek}/><div><strong>{c.title}</strong><p>{c.summary}</p></div></li>)}</ol>:<p className="help">当前结果没有章节。</p>}</section>;}
