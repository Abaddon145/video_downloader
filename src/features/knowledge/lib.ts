import {unified} from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
const markdownParser=unified().use(remarkParse).use(remarkGfm);

export function relativeNotePath(source:string,target:string):string|null{
 try{target=decodeURIComponent(target).replace(/\\/g,'/');}catch{return null;}
 if(target.startsWith('/')||target.includes(':')||/[\u0000-\u001f]/.test(target))return null;
 const path=target.split('#')[0].split('?')[0];if(!path)return source||null;
 const parts=source.split('/').slice(0,-1);
 for(const part of path.split('/')){if(!part||part==='.')continue;if(part==='..'){if(!parts.length)return null;parts.pop();}else parts.push(part);}
 return parts.length?parts.join('/'):null;
}
export function headingSlug(text:string){return text.toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu,'').trim().replace(/\s+/g,'-')||'section';}
export const MAX_OUTLINE_CHARACTERS=256*1024;
export function headings(text:string){
 // ponytail: bound synchronous parsing; use a worker if large notes need an outline.
 if(text.length>MAX_OUTLINE_CHARACTERS)return [];
 const result:{id:string;text:string;depth:number;offset:number}[]=[],body=previewMarkdown(text),base=text.length-body.length;
 const tree=markdownParser.parse(body) as MdNode;remarkKnowledge()(tree);
 const walk=(node:MdNode)=>{if(node.type==='heading')result.push({id:node.data?.hProperties?.id??'section',text:nodeText(node)||'无标题',depth:node.depth??1,offset:base+(node.position?.start.offset??0)});node.children?.forEach(walk);};
 walk(tree);return result;
}
export interface MdNode{position?:{start:{offset?:number}};type:string;value?:string;children?:MdNode[];url?:string;alt?:string;depth?:number;data?:{hProperties?:{id?:string}}}
function nodeText(node:MdNode):string{return node.type==='html'?'':node.alt??node.value??node.children?.map(nodeText).join('')??'';}
export function remarkKnowledge(){return(tree:MdNode)=>{
 const seen=new Map<string,number>();
 const walk=(node:MdNode)=>{
  if(['code','inlineCode','html','link','image'].includes(node.type))return;
  if(!node.children)return;const next:MdNode[]=[];
  for(const child of node.children){if(child.type!=='text'||!child.value){walk(child);next.push(child);continue;}let at=0;const pattern=/(!?)\[\[([^\]\n]+)\]\]/g;for(const m of child.value.matchAll(pattern)){
   if(m.index!>at)next.push({type:'text',value:child.value.slice(at,m.index)});const [target,...alias]=m[2].split('|');const label=alias.join('|')||target;
   next.push(m[1]?{type:'image',url:'yingliu-asset:'+encodeURIComponent(target),alt:label}:{type:'link',url:'yingliu-wiki:'+encodeURIComponent(target),children:[{type:'text',value:label}]});at=m.index!+m[0].length;
  }if(at<child.value.length)next.push({...child,value:child.value.slice(at)});
  }node.children=next;
  if(node.type==='heading'){const slug=headingSlug(nodeText(node)),n=seen.get(slug)??0;seen.set(slug,n+1);node.data={...node.data,hProperties:{...node.data?.hProperties,id:slug+(n?`-${n}`:'')}};}
 };walk(tree);
};}

export function headingFragmentId(fragment:string):string|null{
 try{return headingSlug(decodeURIComponent(fragment.slice(1)));}catch{return null;}
}
export function previewMarkdown(text:string):string{
 return text.replace(/^\uFEFF?---[ \t]*\r?\n(?:[\s\S]*?\r?\n)?---[ \t]*(?:\r?\n|$)/,'');
}
export function safeMarkdownUrl(url:string):string{
 if(/[\u0000-\u0020\u007f]/.test(url))return '';
 if(/^https?:\/\//i.test(url)){try{const parsed=new URL(url);return parsed.hostname?url:'';}catch{return '';}}
 if(url.startsWith('#'))return headingFragmentId(url)===null?'':url;
 if(/^(yingliu-(wiki|asset):|yingliu:\/\/video\/)/.test(url))return url;
 try{const decoded=decodeURIComponent(url);return /^[\\/]/.test(decoded)||/[:\u0000-\u001f\u007f]/.test(decoded)?'':url;}catch{return '';}
}
