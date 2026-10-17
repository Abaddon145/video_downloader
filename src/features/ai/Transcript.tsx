
import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {TranscriptPage} from '../../types/ai';
import {TimeButton} from './VideoChapters';import {aiError} from './lib';
export function Transcript({analysisId,onSeek}:{analysisId:string;onSeek?:(ms:number)=>void}){
 const [page,setPage]=useState(1),[data,setData]=useState<TranscriptPage|null>(null),[error,setError]=useState('');
 useEffect(()=>{setPage(1);setData(null);setError('');},[analysisId]);
 useEffect(()=>{let alive=true;setData(null);invoke<TranscriptPage>('ai_transcript',{id:analysisId,page}).then(v=>{if(alive)setData(v);}).catch(e=>{if(alive)setError(aiError(e));});return()=>{alive=false;};},[analysisId,page]);
 return <section className="ai-transcript" aria-label="带时间码转录">{error&&<p className="error-text" role="alert">{error}</p>}{!data&&!error&&<p className="help" role="status">正在读取转录…</p>}{data&&<><p className="help">{data.total} 段 · 每页最多 50 段</p>{data.items.map(s=><div className="ai-transcript-line" key={s.id}><TimeButton ms={s.startMs} onSeek={onSeek}/><p>{s.text}</p></div>)}{!data.total&&<p className="help">没有语音转录内容。无音轨视频仍可使用画面分析。</p>}{data.total>50&&<div className="pagination"><button className="secondary" disabled={page<=1} onClick={()=>setPage(page-1)}>上一页转录</button><span>{page} / {Math.ceil(data.total/50)}</span><button className="secondary" disabled={page*50>=data.total} onClick={()=>setPage(page+1)}>下一页转录</button></div>}</>}</section>;
}
