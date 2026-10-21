import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {CourseTranscriptPage,TranscriptSegment} from '../../types/ai';
import {TimeButton} from './VideoChapters';
import {aiError,transcriptPageNeedsFollow} from './lib';
import {subtitleMarkdown} from './courseKnowledge';
export function Transcript({itemId,analysisId,onSeek,currentMs=0,onRecognize,running=false,onDraft,onAsk}:{itemId:string;analysisId?:string;onSeek?:(ms:number)=>void;currentMs?:number;onRecognize?:()=>void;running?:boolean;onDraft?:(markdown:string)=>void;onAsk?:(question:string)=>void}){
 const [request,setRequest]=useState<{page:number;at:number|null;query:string}>({page:1,at:null,query:''}),[data,setData]=useState<CourseTranscriptPage|null>(null),[error,setError]=useState(''),[loading,setLoading]=useState(false),[follow,setFollow]=useState(true),[search,setSearch]=useState(''),[selected,setSelected]=useState<TranscriptSegment[]>([]);
 const list=useRef<HTMLDivElement>(null),now=useRef(currentMs);now.current=currentMs;
 useEffect(()=>{setRequest({page:1,at:Math.round(now.current),query:''});setData(null);setError('');setSearch('');setSelected([]);},[itemId,analysisId]);
 useEffect(()=>{const timer=setTimeout(()=>setRequest({page:1,at:search.trim()?null:Math.round(now.current),query:search.trim()}),250);return()=>clearTimeout(timer);},[search,itemId,analysisId]);
 useEffect(()=>{let alive=true;setLoading(true);setError('');invoke<CourseTranscriptPage>('ai_course_transcript',{itemId,page:request.page,currentMs:request.at,query:request.query||null}).then(v=>{if(alive)setData(v);}).catch(e=>{if(alive)setError(aiError(e));}).finally(()=>{if(alive)setLoading(false);});return()=>{alive=false;};},[itemId,analysisId,request]);
 useEffect(()=>{if(follow&&!request.query&&!loading&&data&&transcriptPageNeedsFollow(data,currentMs)){setLoading(true);setRequest({...request,page:data.page,at:Math.round(currentMs)});}},[follow,loading,data,currentMs,request]);
 const active=data?.items.find(s=>currentMs>=s.startMs&&currentMs<s.endMs)?.id;
 useEffect(()=>{if(!follow||!active)return;const container=list.current,row=container?.querySelector<HTMLElement>('[aria-current=true]');if(container&&row){const top=row.getBoundingClientRect().top-container.getBoundingClientRect().top;container.scrollTop+=top-container.clientHeight/2;}},[follow,active]);
 const manual=(page:number)=>{setFollow(false);setRequest({...request,page,at:null});};
 const toggle=(segment:TranscriptSegment)=>setSelected(current=>current.some(s=>s.id===segment.id)?current.filter(s=>s.id!==segment.id):current.length<50?[...current,segment]:current);
 const quote=subtitleMarkdown(itemId,[...selected].sort((a,b)=>a.startMs-b.startMs));
 return <section className="ai-transcript" aria-label="带时间码转录">
  <label className="check-label transcript-follow"><input type="checkbox" checked={follow} disabled={!!search.trim()} onChange={e=>{setFollow(e.target.checked);if(e.target.checked)setRequest({...request,page:1,at:Math.round(currentMs)});}}/>跟随播放位置</label>
  <label>搜索整节课程字幕<input value={search} maxLength={120} onChange={e=>setSearch(e.target.value)} placeholder="输入字幕中的词句"/></label>
  {!!selected.length&&<div className="ai-actions"><span>{selected.length} / 50 段已选</span><button type="button" className="secondary" onClick={()=>void navigator.clipboard.writeText(quote).catch(e=>setError(aiError(e)))}>复制原文与时间链接</button><button type="button" className="secondary" onClick={()=>onDraft?.(quote)}>保存到知识库</button><button type="button" className="secondary" onClick={()=>onAsk?.(`请根据这些课程字幕解释：\n${quote.slice(0,1600)}`)}>用所选字幕提问</button><button type="button" className="secondary" onClick={()=>onAsk?.(`请将这些所选课程字幕整理为章节总结，保留真实时间范围，不补充无来源内容：\n${quote.slice(0,1500)}`)}>整理所选章节</button><button type="button" className="text-button" onClick={()=>setSelected([])}>清除选择</button></div>}
  {error&&<p className="error-text" role="alert">{error}<button className="text-button" onClick={()=>setRequest({...request})}>重新读取字幕</button></p>}
  {loading&&<p className="help" role="status">正在读取字幕…</p>}
  {data&&<><p className="help">{data.source==='subtitle'?'已有字幕':data.source==='whisper'?'本地语音转录':'课程字幕'} · {data.total} 段{request.query?'匹配':' '} · 每页最多 50 段</p>{data.warning&&<details className="transcript-warning"><summary>字幕读取提示</summary><p className="help">{data.warning}</p></details>}
   <div className="ai-transcript-list" ref={list}>{data.items.map(s=><div className="ai-transcript-line" key={s.id} aria-current={currentMs>=s.startMs&&currentMs<s.endMs?true:undefined}><input type="checkbox" aria-label={`选择 ${s.startMs} 毫秒字幕`} checked={selected.some(v=>v.id===s.id)} disabled={selected.length>=50&&!selected.some(v=>v.id===s.id)} onChange={()=>toggle(s)}/><TimeButton ms={s.startMs} onSeek={onSeek}/><p>{s.text}</p></div>)}</div>
   {!data.total&&<div className="ai-empty"><p>没有可用文本字幕时，可以在本地识别课程语音；没有音轨的课程仍可分析画面。</p>{onRecognize&&<button type="button" className="secondary" disabled={running} onClick={onRecognize}>识别这节课程</button>}</div>}
   {data.total>50&&<div className="pagination"><button className="secondary" disabled={data.page<=1||loading} onClick={()=>manual(data.page-1)}>上一页转录</button><span>{data.page} / {Math.ceil(data.total/50)}</span><button className="secondary" disabled={data.page*50>=data.total||loading} onClick={()=>manual(data.page+1)}>下一页转录</button></div>}
  </>}
 </section>;
}
