import {useEffect,useRef,useState,type RefObject} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {PlaybackProgress} from '../../types/learning';
import type {PlaybackHandle,NativeSnapshot,NativeAction} from './playbackHandle';
import {learningError} from './lib';

interface Props {
 itemId:string;session:string;autoplay:boolean;handleRef:RefObject<PlaybackHandle|null>;
 onState:(state:NativeSnapshot)=>void;onError:(message:string)=>void;onShortcut:(action:string)=>void;
}
export function NativeLearningSurface(props:Props){
 const root=useRef<HTMLDivElement>(null),callbacks=useRef(props),[ready,setReady]=useState(false);
 callbacks.current=props;
 useEffect(()=>{
  // Each mounted surface owns a distinct engine, including StrictMode's probe mount.
  const session=`${props.session}-n-${Math.random().toString(36).slice(2,9)}`;
  let alive=true,started=false,frame=0,lastRect='',state:NativeSnapshot={session,itemId:props.itemId,ready:false,position:0,duration:0,rate:1,paused:true,volume:.8,muted:false,ended:false,subtitles:[],subtitleId:null,error:null};
  const accept=(next:NativeSnapshot)=>{if(!alive||next.session!==session||next.itemId!==props.itemId)return;state=next;setReady(next.ready);callbacks.current.onState(next);if(next.error)callbacks.current.onError(next.error);};
  const fail=(e:unknown)=>{if(alive)callbacks.current.onError(learningError(e));};
  const viewport=()=>{
   const r=root.current?.getBoundingClientRect();
   return {left:r?.left??0,top:r?.top??0,width:r?.width??0,height:r?.height??0,
    visible:!!r&&!document.hidden&&!document.querySelector('dialog[open],[role="dialog"][aria-modal="true"]')};
  };
  const events=listen<NativeSnapshot>('learning-native-updated',e=>accept(e.payload));
  const shortcuts=listen<{session:string;itemId:string;action:string}>('learning-native-shortcut',e=>{
   if(alive&&e.payload.session===session&&e.payload.itemId===props.itemId)callbacks.current.onShortcut(e.payload.action);
  });
  const start=Promise.all([events,shortcuts]).then(async()=>{
   if(!alive)return;
   accept(await invoke<NativeSnapshot>('learning_native_start',{itemId:props.itemId,session,viewport:viewport(),autoplay:props.autoplay}));
   started=true;geometry();
  });
  let queue:Promise<unknown>=start;
  const enqueue=<T,>(operation:()=>Promise<T>):Promise<T>=>{
   const next=queue.then(()=>{if(!alive)throw new Error('播放会话已关闭');return operation();});
   queue=next.catch(()=>{});return next;
  };
  const control=(action:NativeAction)=>enqueue(async()=>{const s=await invoke<NativeSnapshot>('learning_native_control',{session,action});accept(s);return s;});
  const set=(action:NativeAction)=>{void control(action).catch(fail);};
  const handle:PlaybackHandle={
   get currentTime(){return state.position},set currentTime(position){set({type:'seek',position});},
   get duration(){return state.duration},
   get playbackRate(){return state.rate},set playbackRate(rate){set({type:'rate',rate});},
   get volume(){return state.volume},set volume(volume){set({type:'volume',volume});},
   get muted(){return state.muted},set muted(muted){set({type:'mute',muted});},
   get paused(){return state.paused},
   play:async()=>{await control({type:'pause',paused:false});},pause:()=>set({type:'pause',paused:true}),control,
   persist:(ended=false)=>enqueue(()=>invoke<PlaybackProgress>('learning_native_save_progress',{session,ended})),
  };
  props.handleRef.current=handle;
  function geometry(){
   if(!alive||!started||frame)return;
   frame=requestAnimationFrame(()=>{frame=0;const rect=viewport(),key=JSON.stringify(rect);if(key===lastRect)return;lastRect=key;void invoke('learning_native_viewport',{session,viewport:rect}).catch(fail);});
  }
  const resize=new ResizeObserver(geometry);if(root.current)resize.observe(root.current);
  const mutation=new MutationObserver(geometry);mutation.observe(document.body,{subtree:true,attributes:true,attributeFilter:['open','class','style','aria-modal'],childList:true});
  window.addEventListener('resize',geometry);window.addEventListener('scroll',geometry,true);document.addEventListener('visibilitychange',geometry);
  void start.catch(fail);
  return()=>{
   alive=false;resize.disconnect();mutation.disconnect();cancelAnimationFrame(frame);
   window.removeEventListener('resize',geometry);window.removeEventListener('scroll',geometry,true);document.removeEventListener('visibilitychange',geometry);
   if(props.handleRef.current===handle)props.handleRef.current=null;
   void invoke('learning_cancel_player',{session}).catch(()=>{});
   void events.then(fn=>fn());void shortcuts.then(fn=>fn());
  };
 },[props.itemId,props.session,props.autoplay,props.handleRef]);
 return <div ref={root} className="learning-native-surface" aria-label="课程视频画面" aria-busy={!ready}>
  {!ready&&<span role="status">正在打开原文件…</span>}
 </div>;
}
