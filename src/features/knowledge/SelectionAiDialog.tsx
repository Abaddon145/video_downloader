import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {LearningDialog} from '../learning/Editors';
import {useFeedback} from '../../components/Feedback';
import {aiError} from '../ai/lib';
import type {AiSettingsView} from '../../types/ai';
import type {NoteSnapshot} from './NoteTools';
import type {SelectionAiAction,SelectionSnapshot} from './SelectionToolbar';
import {applySelectionDraft,selectionMatches} from './SelectionAiDraft';

type Draft={requestId:string;vaultId:string;path:string;sourceHash:string;inputHash:string;markdown:string};
const names={summary:'AI 总结',polish:'AI 润色',review:'创建复习问题'};
const hash=async(text:string)=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text)))].map(b=>b.toString(16).padStart(2,'0')).join('');

export function SelectionAiDialog({action,selection,prepare,current,apply,close}:{action:SelectionAiAction;selection:SelectionSnapshot;prepare:()=>Promise<NoteSnapshot>;current:()=>NoteSnapshot|null;apply:(text:string,snapshot:NoteSnapshot)=>boolean;close:()=>void}){
 const {recordUndo}=useFeedback();
 const [settings,setSettings]=useState<AiSettingsView|null>(null),[consent,setConsent]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState(''),[draft,setDraft]=useState<Draft|null>(null),[source,setSource]=useState<NoteSnapshot|null>(null);
 const active=useRef(true),request=useRef<string|null>(null),ticket=useRef(0);
 useEffect(()=>{active.current=true;void invoke<AiSettingsView>('ai_get_settings').then(v=>{if(active.current)setSettings(v);}).catch(e=>{if(active.current)setError(aiError(e));});return()=>{active.current=false;++ticket.current;if(request.current)void invoke('ai_cancel_note_organize',{requestId:request.current}).catch(()=>{});};},[]);
 const dismiss=()=>{++ticket.current;if(request.current)void invoke('ai_cancel_note_organize',{requestId:request.current}).catch(()=>{});close();};
 async function run(){if(busy||!settings?.hasApiKey||!settings.settings.allowNoteText||!consent)return;const own=++ticket.current,id=`selection-${crypto.randomUUID()}`;request.current=id;setBusy(true);setError('');
  try{const saved=await prepare();if(!active.current||own!==ticket.current)return;if(saved.text!==selection.source||!applySelectionDraft(selection,selection.selection,saved.text))throw Error('笔记内容已改变，请重新选择文字。');
   const result=await invoke<Draft>('ai_organize_note',{request:{requestId:id,vaultId:saved.vaultId,path:saved.path,expectedHash:saved.hash,text:selection.selection,allowNoteText:true,operation:action}});
   if(!active.current||own!==ticket.current)return;
   if(result.requestId!==id||result.vaultId!==saved.vaultId||result.path!==saved.path||result.sourceHash!==saved.hash||result.inputHash!==await hash(selection.selection))throw Error('AI 回执与选区不一致，结果未应用。');
   setSource(saved);setDraft(result);
  }catch(e){if(active.current&&own===ticket.current)setError(aiError(e));}finally{if(request.current===id)request.current=null;if(active.current&&own===ticket.current)setBusy(false);}
 }
 function accept(){if(!source||!draft)return;const now=current();const replacement=applySelectionDraft(selection,draft.markdown,now?.text??'');if(!selectionMatches(source,now)||replacement===null||!apply(replacement,source)){setError('笔记已发生变化，请重新选择文字；AI 结果已保留。');return;}
  const after=current();if(after){recordUndo({label:names[action],undo:async()=>{const now=current();if(!now||now.vaultId!==after.vaultId||now.path!==after.path||now.editVersion!==after.editVersion||now.text!==replacement||!apply(selection.source,now))throw Error('笔记已发生变化，撤销未覆盖后续编辑。');}});}close();
 }
 return <LearningDialog title={names[action]} close={dismiss} busy={false}>
  <p className="help">仅发送下方选中文字给 DeepSeek。可能产生 API 费用；结果须由你确认后才会写入笔记。</p>
  <h3>原文</h3><pre className="note-source-preview">{selection.selection}</pre>
  {!settings?.hasApiKey&&<p className="help">请先在设置中配置 API 密钥。</p>}
  {settings&&!settings.settings.allowNoteText&&<button className="secondary" disabled={busy} onClick={()=>{void invoke<AiSettingsView>('ai_save_settings',{settings:{...settings.settings,allowNoteText:true}}).then(setSettings).catch(e=>setError(aiError(e)));}}>启用笔记文本发送权限</button>}
  <label className="check-label"><input type="checkbox" checked={consent} disabled={busy} onChange={e=>setConsent(e.target.checked)}/>本次允许发送以上选中文字</label>
  {draft&&<><h3>AI 结果</h3><pre className="note-source-preview">{draft.markdown}</pre><details><summary>查看修改对比</summary><div className="note-source-preview"><del>{selection.selection}</del><hr/><ins>{draft.markdown}</ins></div></details></>}
  {error&&<p role="alert" className="error-text">{error}</p>}
  <footer><button className="secondary" onClick={dismiss}>{busy?'取消请求':'关闭'}</button>{draft&&<button className="primary" disabled={busy} onClick={accept}>应用到选区</button>}<button className="primary" disabled={busy||!consent||!settings?.hasApiKey||!settings.settings.allowNoteText||!selection.selection.trim()||new TextEncoder().encode(selection.selection).length>48*1024} onClick={()=>{void run();}}>{busy?'处理中…':draft?'重新生成':'确认并生成'}</button></footer>
 </LearningDialog>;
}
