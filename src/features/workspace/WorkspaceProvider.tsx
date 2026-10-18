import {createContext,useCallback,useContext,useEffect,useMemo,useRef,useState} from 'react';
import type {ReactNode} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {defaultWorkspace,readWorkspace,saveWorkspaceQueue} from './defaults';
import {readPendingWorkspace,writePendingWorkspace,acknowledgeWorkspace} from './pending';
import type {WorkspaceAppearance,WorkspaceLayout,WorkspaceSettings} from './types';
import './workspace.css';
interface WorkspaceContextValue {workspace:WorkspaceSettings;setLayout:(updater:(old:WorkspaceLayout)=>WorkspaceLayout)=>void;setAppearance:(updater:(old:WorkspaceAppearance)=>WorkspaceAppearance)=>void;error:string}
const WorkspaceContext=createContext<WorkspaceContextValue>({workspace:defaultWorkspace(),setLayout:()=>{},setAppearance:()=>{},error:''});
export function WorkspaceProvider({children}:{children:ReactNode}) {
 const [workspace,setWorkspace]=useState(defaultWorkspace),[error,setError]=useState(''),[ready,setReady]=useState(false),[dismissed,setDismissed]=useState(false);
 const [loadFailed,setLoadFailed]=useState(false),[loadAttempt,setLoadAttempt]=useState(0);
 const latest=useRef(workspace),changed=useRef(false),alive=useRef(true);
 const save=useMemo(()=>saveWorkspaceQueue(async value=>{
  if(!('__TAURI_INTERNALS__' in window))return;
  await invoke('workspace_save',{settings:value});
  try {acknowledgeWorkspace(localStorage,value);}catch{/* Rust save succeeded; keep a harmless recovery copy. */}
  if(alive.current)setError('');
 },e=>{if(alive.current){setError(String(e));setDismissed(false);}}),[]);
 useEffect(()=>{
  let live=true,loaded=false;alive.current=true;setError('');setLoadFailed(false);setDismissed(false);
  let pending:WorkspaceSettings|null=null;
  try {pending=readPendingWorkspace(localStorage);}catch{/* Rust remains usable without browser storage. */}
  if(!('__TAURI_INTERNALS__' in window)){
   const initial=pending??defaultWorkspace();latest.current=initial;changed.current=!!pending;setWorkspace(initial);setReady(true);
   return()=>{live=false;alive.current=false;};
  }
  void invoke<unknown>('workspace_get').then(value=>{
   if(!live)return;
   const initial=pending??readWorkspace(value);
   latest.current=initial;changed.current=!!pending;setWorkspace(initial);loaded=true;
  }).catch(e=>{
   if(!live)return;
   if(pending){latest.current=pending;changed.current=true;setWorkspace(pending);loaded=true;}
   setLoadFailed(!pending);setError(String(e));
  }).finally(()=>{if(live)setReady(loaded);});
  return()=>{live=false;alive.current=false;};
 },[loadAttempt]);
 const update=useCallback((next:WorkspaceSettings)=>{
  const validated=readWorkspace(next);
  if(JSON.stringify(validated)!==JSON.stringify(next)){setError('布局数值无效，请检查面板尺寸和可见列');setDismissed(false);return;}
  latest.current=next;changed.current=true;
  try {writePendingWorkspace(localStorage,next);}catch(e){setError(String(e));setDismissed(false);save(next);}
  setWorkspace(next);
 },[save]);
 const setLayout=useCallback((updater:(old:WorkspaceLayout)=>WorkspaceLayout)=>update({...latest.current,currentLayout:updater(latest.current.currentLayout)}),[update]);
 const setAppearance=useCallback((updater:(old:WorkspaceAppearance)=>WorkspaceAppearance)=>update({...latest.current,appearance:updater(latest.current.appearance)}),[update]);
 useEffect(()=>{if(!ready||!changed.current)return;const timer=setTimeout(()=>save(workspace),160);return()=>clearTimeout(timer);},[workspace,ready,save]);
 return <WorkspaceContext.Provider value={{workspace,setLayout,setAppearance,error}}>
  {ready?children:<div className="workspace-loading" role="status">正在恢复工作区…</div>}
  {error&&!dismissed&&<div className="workspace-save-error" role="status" data-native-overlay="true"><strong>{loadFailed?'无法恢复工作区':'布局尚未写入磁盘'}</strong><p>{loadFailed?'读取失败，已暂停布局编辑以保护原有设置。请重新读取。':'请重试保存。有效的待保存布局会在下次启动时恢复。'}</p><details><summary>技术详情</summary><p>{error}</p></details><div><button className="secondary" onClick={()=>loadFailed?setLoadAttempt(n=>n+1):save(latest.current)}>{loadFailed?'重新读取工作区':'重试保存'}</button>{!loadFailed&&<button className="text-button" onClick={()=>setDismissed(true)}>知道了</button>}</div></div>}
 </WorkspaceContext.Provider>;
}
export function useWorkspace(){return useContext(WorkspaceContext);}
