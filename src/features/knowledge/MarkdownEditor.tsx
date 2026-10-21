import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from 'react';
import CodeMirror, { EditorView, type ReactCodeMirrorRef } from '@uiw/react-codemirror';
import { markdown } from '@codemirror/lang-markdown';
import {EditorState, Transaction} from '@codemirror/state';
import {useWorkspace} from '../workspace/WorkspaceProvider';
import {markdownLivePreview, type LivePreviewOptions} from './MarkdownLivePreview';
import {SelectionToolbar, type SelectionAiAction, type SelectionSnapshot} from './SelectionToolbar';
import './livePreview.css';

type MarkdownEditorProps = {
  value: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
  editorRef: RefObject<ReactCodeMirrorRef | null>;
  live?: boolean;
  vaultId?: string;
  path?: string;
  onLink?: LivePreviewOptions['onLink'];
  onExternal?: LivePreviewOptions['onExternal'];
  onVideo?: LivePreviewOptions['onVideo'];
  onSelectionAi?: (action:SelectionAiAction,snapshot:SelectionSnapshot)=>void;
  onInsertTime?: (snapshot:SelectionSnapshot)=>string|null;
};

export function MarkdownEditor({ value, onChange, readOnly = false, editorRef, live = true, vaultId, path, onLink, onExternal, onVideo, onSelectionAi, onInsertTime }: MarkdownEditorProps) {
  const {resolvedTheme}=useWorkspace();
  const [view,setView]=useState<EditorView|null>(null);
  const [selection,setSelection]=useState<{view:EditorView;top:number;left:number}|null>(null);
  const initialValue=useRef(value).current;
  const onChangeRef=useRef(onChange);
  onChangeRef.current=onChange;
  const handlers=useRef({onLink,onExternal,onVideo});
  handlers.current={onLink,onExternal,onVideo};
  const lineBreak=/\r\n|\n|\r/.exec(value)?.[0]??'\n';
  useEffect(()=>{
    if(view&&view.state.sliceDoc()!==value)view.dispatch({changes:{from:0,to:view.state.doc.length,insert:value},annotations:Transaction.addToHistory.of(false)});
  },[view,value]);
  const extensions = useMemo(
    () => [
      markdown(),
      EditorState.lineSeparator.of(lineBreak),
      ...(live?[markdownLivePreview({vaultId,path,onLink:(target,wiki)=>handlers.current.onLink?.(target,wiki),onExternal:url=>handlers.current.onExternal?.(url),onVideo:(item,time)=>handlers.current.onVideo?.(item,time)})]:[]),
      EditorView.lineWrapping,
      EditorView.contentAttributes.of({ 'aria-label': 'Markdown正文' }),
    ],
    [live,lineBreak,vaultId,path],
  );
  const handleChange=useCallback((_next:string,update:{state:EditorState})=>onChangeRef.current(update.state.sliceDoc()),[]);
  const handleUpdate=useCallback((update:{view:EditorView;selectionSet:boolean;docChanged:boolean;viewportChanged:boolean})=>{
    if(!update.selectionSet&&!update.docChanged&&!update.viewportChanged)return;
    const view=update.view,range=view.state.selection.main;
    if(!live||readOnly||view.composing||range.empty){setSelection(null);return;}
    const shell=view.dom.closest<HTMLElement>('.knowledge-live-shell'),coords=view.coordsAtPos(range.from);
    if(!shell||!coords){setSelection(null);return;}
    const box=shell.getBoundingClientRect();
    setSelection({view,top:Math.max(42,coords.top-box.top-43),left:Math.max(8,Math.min(coords.left-box.left,box.width-340))});
  },[live,readOnly]);

  return (
    <div className={`knowledge-live-shell ${live?'is-live':''}`}>
    <SelectionToolbar view={view} readOnly={readOnly} onAi={onSelectionAi} onInsertTime={onInsertTime}/>
    {selection&&<SelectionToolbar compact view={selection.view} readOnly={readOnly} onAi={onSelectionAi} style={{top:selection.top,left:selection.left}}/>}
    <CodeMirror
      ref={editorRef}
      value={initialValue}
      onChange={handleChange}
      onCreateEditor={created=>setView(created)}
      extensions={extensions}
      theme={resolvedTheme}
      height="100%"
      minHeight="420px"
      readOnly={readOnly}
      editable={!readOnly}
      indentWithTab={false}
      className="knowledge-code-editor"
      onUpdate={handleUpdate}
    />
    </div>
  );
}
