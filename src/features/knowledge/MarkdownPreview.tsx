import {useEffect,useState} from 'react';
import Markdown from 'react-markdown';import remarkGfm from 'remark-gfm';import {invoke} from '@tauri-apps/api/core';
import type {KnowledgeAsset,KnowledgeView} from '../../types/knowledge';import {headingFragmentId,previewMarkdown,safeMarkdownUrl,relativeNotePath,remarkKnowledge} from './lib';
function Attachment({vaultId,path,src='',alt=''}:{vaultId:string;path:string;src?:string;alt?:string}){
 const [data,setData]=useState(''),[failed,setFailed]=useState(false);
 useEffect(()=>{let alive=true;setData('');setFailed(false);void(async()=>{
  if(/^https?:/i.test(src)){setFailed(true);return;}
  let target=src;try{if(target.startsWith('yingliu-asset:'))target=decodeURIComponent(target.slice(14));}catch{setFailed(true);return;}
  const candidates=[relativeNotePath(path,target),relativeNotePath('',target)].filter((v,i,all):v is string=>!!v&&all.indexOf(v)===i);
  for(const candidate of candidates){try{const r=await invoke<KnowledgeView<KnowledgeAsset>>('knowledge_asset',{vaultId,path:candidate});if(r.vaultId===vaultId&&/^image\/(png|jpeg|gif|webp|bmp|avif)$/.test(r.data.mime)){if(alive)setData(`data:${r.data.mime};base64,${r.data.base64}`);return;}}catch{/* Try the Vault-relative Obsidian attachment location. */}}
  if(alive)setFailed(true);
 })();return()=>{alive=false;};},[vaultId,path,src]);
 return data?<img src={data} alt={alt} loading="lazy"/>:<span className="knowledge-image-placeholder">{alt||'图片附件'} · {failed?'图片未加载':'正在读取图片…'}</span>;
}
export function MarkdownPreview({value,vaultId,path,onLink,onExternal,onVideo}:{value:string;vaultId:string;path:string;onLink:(target:string,wiki:boolean)=>void;onExternal:(url:string)=>void;onVideo?:(itemId:string,timeMs:number)=>void}){
 return <article className="knowledge-preview"><Markdown skipHtml remarkPlugins={[remarkGfm,remarkKnowledge]} urlTransform={safeMarkdownUrl} components={{
  a:({href='',children})=><a href={href||undefined} onClick={e=>{e.preventDefault();if(href.startsWith('yingliu-wiki:')){try{onLink(decodeURIComponent(href.slice(13)),true);}catch{/* Invalid encoded link stays inert. */}}else if(href.startsWith('#')){const id=headingFragmentId(href);if(id)e.currentTarget.closest('article')?.querySelectorAll<HTMLElement>('[id]').forEach(node=>{if(node.id===id)node.scrollIntoView({block:'nearest'});});}else if(/^https?:\/\//i.test(href))onExternal(href);else if(href.startsWith('yingliu://video/')){try{const u=new URL(href),time=Number(u.searchParams.get('t')??0),item=decodeURIComponent(u.pathname.slice(1));if(onVideo&&item&&Number.isSafeInteger(time)&&time>=0)onVideo(item,time);}catch{/* Unknown media link is not executable. */}}else if(href)onLink(href,false);}}>{children}</a>,
  img:({src,alt})=><Attachment vaultId={vaultId} path={path} src={src} alt={alt}/>,
 }}>{previewMarkdown(value)}</Markdown></article>;
}
