
import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Send,Sparkles} from 'lucide-react';
import type {QuestionReply} from '../../types/ai';
import {aiError,timeLabel} from './lib';
export function VideoAsk({analysisId,onSeek,currentMs}:{analysisId:string;onSeek?:(ms:number)=>void;currentMs?:number}){
 const [question,setQuestion]=useState(''),[answer,setAnswer]=useState<QuestionReply|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState('');const epoch=useRef(0);
 useEffect(()=>{epoch.current++;setAnswer(null);setError('');setQuestion('');setBusy(false);return()=>{epoch.current++;};},[analysisId]);
 async function ask(explain=false){const ticket=++epoch.current;setBusy(true);setError('');try{const reply=await invoke<QuestionReply>(explain?'ai_explain_segment':'ai_ask_video',explain?{id:analysisId,timeMs:Math.round(currentMs??0)}:{id:analysisId,question});if(ticket===epoch.current&&reply.analysisId===analysisId)setAnswer(reply);}catch(e){if(ticket===epoch.current)setError(aiError(e));}finally{if(ticket===epoch.current)setBusy(false);}}
 return <section className="ai-video-ask"><div className="ai-section-heading"><h3><Sparkles size={16}/>问这个视频</h3>{currentMs!==undefined&&<button className="text-button" disabled={busy} onClick={()=>void ask(true)}>解释这一段</button>}</div><form onSubmit={e=>{e.preventDefault();void ask();}}><label className="sr-only" htmlFor={`video-question-${analysisId}`}>视频问题</label><input id={`video-question-${analysisId}`} value={question} maxLength={2000} disabled={busy} onChange={e=>setQuestion(e.target.value)} placeholder="例如：哪里讲到了 GPU Collision？"/><button className="primary" disabled={busy||!question.trim()} aria-label="发送视频问题"><Send size={16}/>{busy?'正在查找…':'提问'}</button></form><p className="help">只发送检索到的相关字幕和必要画面；回答依据以原视频为准。</p>{error&&<p className="error-text" role="alert">{error}</p>}{answer&&<div className="ai-answer" aria-live="polite"><p>{answer.answer}</p><div className="ai-tags">{answer.references.map((r,i)=>onSeek?<button className="secondary" key={i} onClick={()=>onSeek(r.startMs)}>▶ {timeLabel(r.startMs)} · {r.label}</button>:<span key={i}>{timeLabel(r.startMs)} · {r.label}</span>)}</div></div>}</section>;
}
