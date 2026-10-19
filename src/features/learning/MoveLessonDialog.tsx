import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {LearningItem,LibraryHome} from '../../types/learning';
import {LearningDialog} from './Editors';
import {learningError} from './lib';
export function MoveLessonDialog({item,close,move}:{item:LearningItem;close:()=>void;move:(seriesId:string)=>Promise<void>}){
 const [query,setQuery]=useState(''),[page,setPage]=useState(1),[home,setHome]=useState<LibraryHome|null>(null),[target,setTarget]=useState(''),[busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{let live=true;const timer=setTimeout(()=>{void invoke<LibraryHome>('learning_home',{query,page,category:null,favorite:false,sort:'title'}).then(data=>{if(live){setHome(data);setError('');}}).catch(e=>{if(live)setError(learningError(e));});},150);return()=>{live=false;clearTimeout(timer);};},[query,page]);
 return <LearningDialog title="移动课程" close={close} busy={busy}><form onSubmit={async e=>{e.preventDefault();setBusy(true);try{await move(target);close();}catch(e){setError(learningError(e));}finally{setBusy(false);}}}>
  <p>移动“{item.title}”的学习库归属；保留本地视频、学习进度和笔记。</p>
  <label>搜索目标系列<input value={query} onChange={e=>{setQuery(e.target.value);setPage(1);setTarget('');}}/></label>
  <label>目标系列<select aria-label="目标系列" required value={target} onChange={e=>setTarget(e.target.value)}><option value="">选择系列</option>{home?.series.filter(s=>s.id!==item.seriesId).map(s=><option key={s.id} value={s.id}>{s.title}</option>)}</select></label>
  {home&&home.total>50&&<div className="learning-pagination"><button type="button" className="secondary" disabled={page===1} onClick={()=>{setPage(p=>p-1);setTarget('');}}>上一页</button><span>{page} / {Math.ceil(home.total/50)}</span><button type="button" className="secondary" disabled={page*50>=home.total} onClick={()=>{setPage(p=>p+1);setTarget('');}}>下一页</button></div>}
  {error&&<p className="error-text" role="alert">{error}</p>}<footer><button type="button" className="secondary" disabled={busy} onClick={close}>取消</button><button className="primary" disabled={busy||!target}>确认移动</button></footer>
 </form></LearningDialog>;
}
