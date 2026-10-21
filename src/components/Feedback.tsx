import {createContext,useCallback,useContext,useEffect,useRef,useState,type ReactNode} from 'react';
import {CheckCircle2,AlertCircle,X,Undo2} from 'lucide-react';
import {UndoManager,type UndoEntry} from '../features/undo/UndoManager';
type Notice={id:number;message:string;action?:{label:string;run:()=>void|Promise<unknown>}};
interface Feedback {notify:(message:string,action?:Notice['action'])=>void;recordUndo:(entry:UndoEntry)=>void;undo:()=>Promise<void>;canUndo:boolean}
const Context=createContext<Feedback>({notify:()=>{},recordUndo:()=>{},undo:async()=>{},canUndo:false});
export const useFeedback=()=>useContext(Context);
export function FeedbackProvider({children}:{children:ReactNode}){
 const [notice,setNotice]=useState<Notice|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false),[,refresh]=useState(0);
 const manager=useRef(new UndoManager()),serial=useRef(0);
 useEffect(()=>manager.current.subscribe(()=>refresh(n=>n+1)),[]);
 const notify=useCallback((message:string,action?:Notice['action'])=>{setNotice({id:++serial.current,message,action});setError('');},[]);
 const undoEntry=useCallback(async(entry:UndoEntry):Promise<void>=>{const ticket=serial.current;try{if(await manager.current.undo(entry)&&serial.current===ticket)notify(`已撤销：${entry.label}`);}catch(e){if(serial.current!==ticket)return;notify(`未能撤销：${entry.label}`,{label:'重试撤销',run:()=>undoEntry(entry)});setError(String(e));}},[notify]);
 const undo=useCallback(async()=>{const entry=manager.current.peek();if(entry)await undoEntry(entry);},[undoEntry]);
 const recordUndo=useCallback((entry:UndoEntry)=>{manager.current.push(entry);notify(`已完成：${entry.label}`,{label:'撤销',run:()=>undoEntry(entry)});},[notify,undoEntry]);
 useEffect(()=>{if(!notice||notice.action||busy)return;const timer=setTimeout(()=>setNotice(null),7000);return()=>clearTimeout(timer);},[notice?.id,busy]);
 const execute=async()=>{if(!notice?.action||busy)return;setBusy(true);setError('');try{await notice.action.run();}catch(e){setError(String(e));}finally{setBusy(false);}};
 return <Context.Provider value={{notify,recordUndo,undo,canUndo:!!manager.current.peek()&&!manager.current.busy}}>{children}{notice&&<div className="toast feedback-toast" role={error?'alert':'status'}><span aria-hidden="true">{error?<AlertCircle size={18}/>:<CheckCircle2 size={18}/>}</span><div><span>{notice.message}</span>{error&&<details><summary>操作未完成，当前数据保留</summary><p>{error}</p></details>}</div>{notice.action&&<button className="secondary" disabled={busy||manager.current.busy} onClick={()=>{void execute();}}><Undo2 size={14}/>{busy?'正在执行…':notice.action.label}</button>}<button className="icon-button" aria-label="关闭提示" disabled={busy} onClick={()=>setNotice(null)}><X size={16}/></button></div>}</Context.Provider>;
}
