import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {CourseTranscriptPage} from '../../types/ai';
import {TimeButton} from './VideoChapters';
import {aiError,transcriptPageNeedsFollow} from './lib';
export function Transcript({itemId,analysisId,onSeek,currentMs=0,onRecognize,running=false}:{itemId:string;analysisId?:string;onSeek?:(ms:number)=>void;currentMs?:number;onRecognize?:()=>void;running?:boolean}){
 const [request,setRequest]=useState<{page:number;at:number|null}>({page:1,at:null}),[data,setData]=useState<CourseTranscriptPage|null>(null),[error,setError]=useState(''),[loading,setLoading]=useState(false),[follow,setFollow]=useState(true);
 const list=useRef<HTMLDivElement>(null),now=useRef(currentMs);now.current=currentMs;
 useEffect(()=>{setRequest({page:1,at:Math.round(now.current)});setData(null);setError('');},[itemId,analysisId]);
 useEffect(()=>{let alive=true;setLoading(true);setError('');invoke<CourseTranscriptPage>('ai_course_transcript',{itemId,page:request.page,currentMs:request.at}).then(v=>{if(alive)setData(v);}).catch(e=>{if(alive)setError(aiError(e));}).finally(()=>{if(alive)setLoading(false);});return()=>{alive=false;};},[itemId,analysisId,request]);
 useEffect(()=>{if(follow&&!loading&&data&&transcriptPageNeedsFollow(data,currentMs)){setLoading(true);setRequest({page:data.page,at:Math.round(currentMs)});}},[follow,loading,data,currentMs]);
 const active=data?.items.find(s=>currentMs>=s.startMs&&currentMs<s.endMs)?.id;
 useEffect(()=>{if(!follow||!active)return;const container=list.current,row=container?.querySelector<HTMLElement>('[aria-current=true]');if(container&&row){const top=row.getBoundingClientRect().top-container.getBoundingClientRect().top;container.scrollTop+=top-container.clientHeight/2;}},[follow,active]);
 const manual=(page:number)=>{setFollow(false);setRequest({page,at:null});};
 return <section className="ai-transcript" aria-label="带时间码转录">
  <label className="check-label transcript-follow"><input type="checkbox" checked={follow} onChange={e=>{setFollow(e.target.checked);if(e.target.checked)setRequest({page:1,at:Math.round(currentMs)});}}/>跟随播放位置</label>
  {error&&<p className="error-text" role="alert">{error}<button className="text-button" onClick={()=>setRequest({...request})}>重新读取字幕</button></p>}
  {loading&&<p className="help" role="status">正在读取字幕…</p>}
  {data&&<><p className="help">{data.source==='subtitle'?'已有字幕':data.source==='whisper'?'本地语音转录':'课程字幕'} · {data.total} 段 · 每页最多 50 段</p>{data.warning&&<details className="transcript-warning"><summary>字幕读取提示</summary><p className="help">{data.warning}</p></details>}
   <div className="ai-transcript-list" ref={list}>{data.items.map(s=><div className="ai-transcript-line" key={s.id} aria-current={currentMs>=s.startMs&&currentMs<s.endMs?true:undefined}><TimeButton ms={s.startMs} onSeek={onSeek}/><p>{s.text}</p></div>)}</div>
   {!data.total&&<div className="ai-empty"><p>没有可用文本字幕时，可以在本地识别课程语音；没有音轨的课程仍可分析画面。</p>{onRecognize&&<button type="button" className="secondary" disabled={running} onClick={onRecognize}>识别这节课程</button>}</div>}
   {data.total>50&&<div className="pagination"><button className="secondary" disabled={data.page<=1||loading} onClick={()=>manual(data.page-1)}>上一页转录</button><span>{data.page} / {Math.ceil(data.total/50)}</span><button className="secondary" disabled={data.page*50>=data.total||loading} onClick={()=>manual(data.page+1)}>下一页转录</button></div>}
  </>}
 </section>;
}
