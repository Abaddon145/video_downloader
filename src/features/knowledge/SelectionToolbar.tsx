import type {EditorView} from '@codemirror/view';
import {applySelectionAction,rawSelection, type SelectionAction} from './MarkdownLivePreview';

export type SelectionSnapshot={source:string;selection:string;from:number;to:number};
export type SelectionAiAction='summary'|'polish'|'review';
type Props={view:EditorView|null;readOnly?:boolean;compact?:boolean;style?:React.CSSProperties;onAi?:(action:SelectionAiAction,snapshot:SelectionSnapshot)=>void;onInsertTime?:(snapshot:SelectionSnapshot)=>string|null};

export function selectionSnapshot(view:EditorView):SelectionSnapshot{
 return rawSelection(view.state);
}

export function SelectionToolbar({view,readOnly=false,compact=false,style,onAi,onInsertTime}:Props){
 const disabled=readOnly||!view||view.composing;
 const execute=(action:SelectionAction)=>{
  if(disabled||!view)return;
  let target='';
  if(action==='link'||action==='image'){
   const input=window.prompt(action==='link'?'链接地址':'知识库内图片路径');
   if(!input)return;
   target=input.trim();
   if(!target||/[\s\u0000-\u001f]/.test(target)||action==='image'&&/^(?:https?:|[\/])/.test(target))return;
  }
  if(action==='time'){
   target=onInsertTime?.(selectionSnapshot(view))??'';
   if(!target)return;
  }
  applySelectionAction(view,action,target);
 };
 const ai=(action:SelectionAiAction)=>{if(disabled||!view||view.state.selection.main.empty)return;onAi?.(action,selectionSnapshot(view));};
 return <div className={`knowledge-selection-toolbar ${compact?'compact':''}`} role="toolbar" aria-label={compact?'选中文字操作':'Markdown格式工具'} style={style} onMouseDown={e=>{if(e.target instanceof HTMLButtonElement)e.preventDefault();}}>
  {!compact&&<button type="button" disabled={disabled} onClick={()=>execute('heading')}>标题</button>}
  <button type="button" disabled={disabled} onClick={()=>execute('bold')} aria-label="加粗"><strong>B</strong></button>
  <button type="button" disabled={disabled} onClick={()=>execute('italic')} aria-label="斜体"><em>I</em></button>
  <button type="button" disabled={disabled} onClick={()=>execute('link')}>链接</button>
  <button type="button" disabled={disabled} onClick={()=>execute('quote')}>引用</button>
  {!compact&&<><button type="button" disabled={disabled} onClick={()=>execute('image')}>图片</button><button type="button" disabled={disabled||!onInsertTime} onClick={()=>execute('time')}>时间点</button></>}
  {compact&&onAi&&<><button type="button" disabled={disabled} onClick={()=>ai('summary')}>AI 总结</button><button type="button" disabled={disabled} onClick={()=>ai('polish')}>AI 润色</button><button type="button" disabled={disabled} onClick={()=>ai('review')}>复习问题</button></>}
 </div>;
}
