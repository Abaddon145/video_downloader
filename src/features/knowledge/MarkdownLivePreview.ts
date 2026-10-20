import {EditorSelection, type Extension} from '@codemirror/state';
import {Decoration, EditorView, ViewPlugin, WidgetType, type DecorationSet, type ViewUpdate} from '@codemirror/view';
import {invoke} from '@tauri-apps/api/core';
import type {KnowledgeAsset, KnowledgeView} from '../../types/knowledge';
import {MAX_OUTLINE_CHARACTERS, relativeNotePath, safeMarkdownUrl} from './lib.ts';

export type SelectionAction='heading'|'bold'|'italic'|'quote'|'link'|'image'|'time';
export type LinkTarget={target:string;wiki:boolean;kind:'link'|'video'};
type Span={from:number;to:number};
type StyledSpan=Span&{className:string};
type PreviewLink=Span&LinkTarget;
type PreviewImage=Span&{src:string;alt:string};
export type PreviewLine={kind:string;hidden:Span[];styled:StyledSpan[];links:PreviewLink[];images:PreviewImage[]};

export function previewLine(text:string,offset:number,active:boolean,inCode=false):PreviewLine{
 const result:PreviewLine={kind:'paragraph',hidden:[],styled:[],links:[],images:[]};
 if(active)return result;
 const hide=(from:number,to:number)=>{if(to>from)result.hidden.push({from:offset+from,to:offset+to});};
 const style=(from:number,to:number,className:string)=>{if(to>from)result.styled.push({from:offset+from,to:offset+to,className});};
 const fence=/^\s{0,3}(`{3,}|~{3,})/.exec(text);
 if(fence){result.kind='code-fence';return result;}
 if(inCode){result.kind='code';return result;}
 const heading=/^(#{1,6})[ \t]+/.exec(text);
 const quote=/^([ \t]*>[ \t]?)/.exec(text);
 const list=/^([ \t]*(?:[-*+]|\d+[.)])[ \t]+)/.exec(text);
 if(heading){result.kind=`heading-${heading[1].length}`;hide(0,heading[0].length);}
 else if(quote){result.kind='quote';hide(0,quote[0].length);}
 else if(list){result.kind='list';hide(0,list[0].length);}
 else if(/^\s*\|.*\|\s*$/.test(text))result.kind='table';

 const occupied:Span[]=[];
 const reserve=(from:number,to:number)=>{if(occupied.some(s=>from<s.to&&to>s.from))return false;occupied.push({from,to});return true;};
 for(const m of text.matchAll(/(!?)\[\[([^\]\n]+)\]\]/g)){
  const start=m.index!,end=start+m[0].length,bar=m[2].indexOf('|'),target=(bar<0?m[2]:m[2].slice(0,bar)).trim();
  if(!target||!reserve(start,end))continue;
  const label=bar<0?target:m[2].slice(bar+1);
  if(m[1]){result.images.push({from:offset+start,to:offset+end,src:target,alt:label});continue;}
  hide(start,start+(bar<0?2:3+bar));hide(end-2,end);
  const labelStart=bar<0?start+2:start+3+bar;
  style(labelStart,end-2,'knowledge-live-link');
  result.links.push({from:offset+labelStart,to:offset+end-2,target,wiki:true,kind:'link'});
 }
 for(const m of text.matchAll(/(!?)\[([^\]\n]+)\]\(([^)\s]+)\)/g)){
  const start=m.index!,end=start+m[0].length,target=safeMarkdownUrl(m[3]);
  if(!target||!reserve(start,end))continue;
  if(m[1]){result.images.push({from:offset+start,to:offset+end,src:target,alt:m[2]});continue;}
  const labelStart=start+1,labelEnd=labelStart+m[2].length;
  hide(start,labelStart);hide(labelEnd,end);style(labelStart,labelEnd,'knowledge-live-link');
  result.links.push({from:offset+labelStart,to:offset+labelEnd,target,wiki:false,kind:target.startsWith('yingliu://video/')?'video':'link'});
 }
 for(const m of text.matchAll(/(?<!\\)(\*\*|__|\*|_)([^\n]+?)\1/g)){
  const start=m.index!,end=start+m[0].length,marker=m[1];
  if(!reserve(start,end))continue;
  hide(start,start+marker.length);hide(end-marker.length,end);
  style(start+marker.length,end-marker.length,marker.length===2?'knowledge-live-bold':'knowledge-live-italic');
 }
 for(const m of text.matchAll(/(?<!\\)(`+)([^`\n]+?)\1/g)){
  const start=m.index!,end=start+m[0].length;
  if(!reserve(start,end))continue;
  hide(start,start+m[1].length);hide(end-m[1].length,end);style(start+m[1].length,end-m[1].length,'knowledge-live-inline-code');
 }
 return result;
}

export function formatSelection(source:string,from:number,to:number,action:SelectionAction,target=''){
 const selected=source.slice(from,to);
 let insert='',anchor=from,head=from;
 if(action==='bold'||action==='italic'||action==='link'||action==='image'||action==='time'){
  const before=action==='bold'?'**':action==='italic'?'*':action==='image'?'![':'[';
  const after=action==='bold'?'**':action==='italic'?'*':action==='link'?`](${target})`:action==='image'?`](${target})`:']('+target+')';
  insert=before+selected+after;anchor=from+before.length;head=anchor+selected.length;
 }else{
  const lineStart=source.lastIndexOf('\n',Math.max(0,from-1))+1;
  const nextBreak=source.indexOf('\n',Math.max(from,to-1));
  const lineEnd=nextBreak<0?source.length:nextBreak;
  const old=source.slice(lineStart,lineEnd);
  const prefix=action==='heading'?'# ':'> ';
  insert=old.split('\n').map(line=>prefix+line).join('\n');
  anchor=lineStart+prefix.length+(from-lineStart);head=lineStart+prefix.length+(to-lineStart)+prefix.length*(old.slice(0,to-lineStart).match(/\n/g)?.length??0);
  from=lineStart;to=lineEnd;
 }
 return {from,to,insert,anchor,head};
}

export function applySelectionAction(view:EditorView,action:SelectionAction,target=''){
 if(view.state.readOnly||view.composing)return false;
 const transaction=view.state.changeByRange(range=>{
  if(action==='heading'||action==='quote'){
   const first=view.state.doc.lineAt(range.from).number,last=view.state.doc.lineAt(Math.max(range.from,range.to-1)).number;
   const changes=view.state.changes(Array.from({length:last-first+1},(_,i)=>({from:view.state.doc.line(first+i).from,insert:action==='heading'?'# ':'> '})));
   return {changes,range:range.map(changes)};
  }
  const before=action==='bold'?'**':action==='italic'?'*':action==='image'?'![':'[';
  const after=action==='bold'?'**':action==='italic'?'*':`](${target})`;
  return {changes:[{from:range.from,insert:before},{from:range.to,insert:after}],range:EditorSelection.range(range.from+before.length,range.to+before.length)};
 });
 view.dispatch(transaction);view.focus();return true;
}
export function rawSelection(state:import('@codemirror/state').EditorState){
 const {from,to}=state.selection.main,source=state.sliceDoc(),selection=state.sliceDoc(from,to),offset=state.sliceDoc(0,from).length;
 return {source,selection,from:offset,to:offset+selection.length};
}

class ImageWidget extends WidgetType{
 readonly src:string;readonly alt:string;readonly vaultId?:string;readonly path?:string;
 constructor(src:string,alt:string,vaultId?:string,path?:string){super();this.src=src;this.alt=alt;this.vaultId=vaultId;this.path=path;}
 eq(other:ImageWidget){return this.src===other.src&&this.alt===other.alt&&this.vaultId===other.vaultId&&this.path===other.path;}
 toDOM(){
  const span=document.createElement('span');span.className='knowledge-live-image';span.textContent=this.alt||'图片附件';
  if(!this.vaultId||!this.path||/^https?:/i.test(this.src))return span;
  let target=this.src;
  try{if(target.startsWith('yingliu-asset:'))target=decodeURIComponent(target.slice(14));}catch{return span;}
  const candidates=[relativeNotePath(this.path,target),relativeNotePath('',target)].filter((p,i,all):p is string=>!!p&&all.indexOf(p)===i);
  void(async()=>{for(const path of candidates){try{
   const response=await invoke<KnowledgeView<KnowledgeAsset>>('knowledge_asset',{vaultId:this.vaultId,path});
   if(response.vaultId!==this.vaultId||!/^image\/(png|jpeg|gif|webp|bmp|avif)$/.test(response.data.mime))continue;
   const img=document.createElement('img');img.src=`data:${response.data.mime};base64,${response.data.base64}`;img.alt=this.alt;img.loading='lazy';
   if(span.isConnected)span.replaceChildren(img);return;
  }catch{/* Try the Vault-relative attachment. */}}})();
  return span;
 }
 ignoreEvent(){return true;}
}

export type LivePreviewOptions={vaultId?:string;path?:string;onLink?:(target:string,wiki:boolean)=>void;onExternal?:(url:string)=>void;onVideo?:(itemId:string,timeMs:number)=>void};
export function markdownLivePreview(options:LivePreviewOptions={}):Extension{
 const linkAt=(event:Event)=>{const element=event.target instanceof Element?event.target.closest<HTMLElement>('[data-knowledge-target]'):null;return element;};
 return [ViewPlugin.fromClass(class{
  decorations:DecorationSet;
  readonly view:EditorView;
  constructor(view:EditorView){this.view=view;this.decorations=this.build();}
  update(update:ViewUpdate){if(update.docChanged||update.selectionSet||update.viewportChanged||update.view.composing)this.decorations=this.build();}
  build(){
   const view=this.view,doc=view.state.doc;
   // ponytail: large notes stay raw; use incremental parsing if live rendering beyond this limit matters.
   if(doc.length>MAX_OUTLINE_CHARACTERS)return Decoration.none;
   const ranges:{from:number;to:number;value:Decoration}[]=[];
   const frontmatter=/^\uFEFF?---[ \t]*\r?\n[\s\S]*?\r?\n---[ \t]*(?:\r?\n|$)/.exec(doc.toString())?.[0].length??0;
   let fence='';
   for(let i=1;i<=doc.lines;i++){
    const line=doc.line(i),match=/^\s{0,3}(`{3,}|~{3,})/.exec(line.text);
    if(match){if(!fence)fence=match[1][0];else if(match[1][0]===fence)fence='';}
    if(!view.visibleRanges.some(r=>line.from<=r.to&&line.to>=r.from))continue;
    if(line.from<frontmatter)continue;
    const active=view.composing||view.state.selection.ranges.some(r=>r.from<=line.to&&r.to>=line.from);
    const info=previewLine(line.text,line.from,active,!!fence&&!match);
    if(info.kind!=='paragraph')ranges.push({from:line.from,to:line.from,value:Decoration.line({class:'knowledge-live-'+info.kind})});
    for(const hidden of info.hidden)ranges.push({...hidden,value:Decoration.replace({})});
    for(const styled of info.styled)ranges.push({...styled,value:Decoration.mark({class:styled.className})});
    for(const link of info.links)ranges.push({from:link.from,to:link.to,value:Decoration.mark({attributes:{'data-knowledge-target':link.target,'data-knowledge-wiki':String(link.wiki),'data-knowledge-kind':link.kind}})});
    for(const item of info.images)ranges.push({from:item.from,to:item.to,value:Decoration.replace({widget:new ImageWidget(item.src,item.alt,options.vaultId,options.path)})});
   }
   return Decoration.set(ranges.sort((a,b)=>a.from-b.from||a.to-b.to).map(r=>r.value.range(r.from,r.to)),true);
  }
 },{decorations:v=>v.decorations}),EditorView.domEventHandlers({
  mousedown(event){if((event.ctrlKey||event.metaKey)&&linkAt(event)){event.preventDefault();return true;}return false;},
  click(event,view){
   if(!(event.ctrlKey||event.metaKey)||view.composing||!view.state.selection.main.empty)return false;
   const element=linkAt(event);if(!element)return false;
   const target=element.dataset.knowledgeTarget??'';if(!target)return false;
   event.preventDefault();
   if(element.dataset.knowledgeWiki==='true')options.onLink?.(target,true);
   else if(/^https?:\/\//i.test(target))options.onExternal?.(target);
   else if(target.startsWith('yingliu://video/')){try{const url=new URL(target),time=Number(url.searchParams.get('t')??0),item=decodeURIComponent(url.pathname.slice(1));if(item&&Number.isSafeInteger(time)&&time>=0)options.onVideo?.(item,time);}catch{/* Invalid time link stays inert. */}}
   else options.onLink?.(target,false);
   return true;
  },
 })];
}
