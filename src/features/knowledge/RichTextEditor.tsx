import {useEffect,useMemo,useRef,useState} from 'react';
import {Plugin,PluginKey} from '@tiptap/pm/state';
import {EditorContent,useEditor} from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import type {ReactCodeMirrorRef} from '@uiw/react-codemirror';
import {MarkdownEditor} from './MarkdownEditor';
import {fromRichText,toRichText,type RichTextResult} from './MarkdownRoundTrip';
import './richText.css';

type Props={value:string;onChange:(value:string)=>void;readOnly?:boolean;onFallback?:(reason:string)=>void};
const extensions=[StarterKit.configure({blockquote:false,bulletList:false,code:false,codeBlock:false,dropcursor:false,gapcursor:false,hardBreak:false,horizontalRule:false,listItem:false,listKeymap:false,link:false,orderedList:false,strike:false,trailingNode:false,underline:false})];

export function RichTextEditor({value,onChange,readOnly=false,onFallback}:Props){
 const [rejected,setRejected]=useState('');
 const result=useMemo(()=>toRichText(value),[value]);
 const current=useRef<RichTextResult>(result),change=useRef(onChange),fallback=useRef(onFallback),lastEmitted=useRef<string|null>(null),sourceRef=useRef<ReactCodeMirrorRef|null>(null);
 current.current=result;change.current=onChange;fallback.current=onFallback;
 const editor=useEditor({extensions,content:result.mode==='rich'?result.doc:undefined,editable:!readOnly,immediatelyRender:false,onUpdate:({editor})=>{
  const baseline=current.current;if(baseline.mode!=='rich')return;
  const next=fromRichText(editor.getJSON(),baseline);
  if(next===null){fallback.current?.('富文本产生了无法安全写回的内容，请使用 Markdown 源码编辑');return;}
  if(next!==baseline.value){lastEmitted.current=next;change.current(next);}
 }});
 useEffect(()=>{if(!editor)return;const key=new PluginKey('yingliu-safe-markdown');editor.registerPlugin(new Plugin({key,filterTransaction(transaction){if(!transaction.docChanged)return true;const baseline=current.current;if(baseline.mode!=='rich')return false;if(fromRichText(transaction.doc.toJSON(),baseline)!==null){setRejected('');return true;}setRejected('此格式无法安全保存，本次格式更改未应用；原笔记保持完整。请改用源码编辑。');return false;}}));return()=>{if(!editor.isDestroyed)editor.unregisterPlugin(key);};},[editor]);
 useEffect(()=>{if(!editor||result.mode!=='rich'||lastEmitted.current===value)return;editor.commands.setContent(result.doc,{emitUpdate:false});},[editor,result,value]);
 useEffect(()=>{editor?.setEditable(!readOnly);},[editor,readOnly]);
 useEffect(()=>{if(result.mode==='source')onFallback?.(result.reason);},[result.mode,result.mode==='source'?result.reason:'',onFallback]);
 if(result.mode==='source')return <div className="knowledge-rich-fallback"><p role="status">此笔记包含需要保留的 Markdown 语法，已切换源码编辑。{result.reason}</p><MarkdownEditor value={value} onChange={onChange} readOnly={readOnly} editorRef={sourceRef}/></div>;
 return <div className="knowledge-rich-editor">{rejected&&<p role="alert" className="error-text">{rejected}</p>}<div className="knowledge-rich-toolbar" role="toolbar" aria-label="富文本格式">
  <button type="button" disabled={!editor||readOnly} onClick={()=>editor?.chain().focus().setParagraph().run()} aria-label="段落">正文</button>
  <button type="button" disabled={!editor||readOnly} onClick={()=>editor?.chain().focus().toggleHeading({level:1}).run()} aria-label="一级标题">H1</button>
  <button type="button" disabled={!editor||readOnly} onClick={()=>editor?.chain().focus().toggleHeading({level:2}).run()} aria-label="二级标题">H2</button>
  <button type="button" disabled={!editor||readOnly} onClick={()=>editor?.chain().focus().toggleBold().run()} aria-label="粗体"><strong>B</strong></button>
  <button type="button" disabled={!editor||readOnly} onClick={()=>editor?.chain().focus().toggleItalic().run()} aria-label="斜体"><em>I</em></button>
 </div><EditorContent editor={editor} aria-label="富文本正文" className="knowledge-rich-content"/></div>;
}
