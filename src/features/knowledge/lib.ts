export function relativeNotePath(source:string,target:string):string|null{
 try{target=decodeURIComponent(target).replace(/\\/g,'/');}catch{return null;}
 if(target.startsWith('/')||target.includes(':')||/[\u0000-\u001f]/.test(target))return null;
 const path=target.split('#')[0].split('?')[0];if(!path)return source||null;
 const parts=source.split('/').slice(0,-1);
 for(const part of path.split('/')){if(!part||part==='.')continue;if(part==='..'){if(!parts.length)return null;parts.pop();}else parts.push(part);}
 return parts.length?parts.join('/'):null;
}
export function headingSlug(text:string){return text.toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu,'').trim().replace(/\s+/g,'-')||'section';}
export function headings(text:string){
 const result:{id:string;text:string;depth:number;offset:number}[]=[],seen=new Map<string,number>();const body=previewMarkdown(text);let offset=text.length-body.length,fence='';
 for(const line of body.split('\n')){
  const marker=line.trimStart().match(/^(`{3,}|~{3,})/);if(marker){if(!fence)fence=marker[1][0];else if(marker[1][0]===fence)fence='';offset+=line.length+1;continue;}
  const match=!fence&&line.match(/^(#{1,6})\s+(.+?)(?:\s+#+\s*)?$/);
  if(match){const title=match[2].replace(/[*_`]/g,'');const slug=headingSlug(title),count=seen.get(slug)??0;seen.set(slug,count+1);result.push({id:slug+(count?`-${count}`:''),text:title,depth:match[1].length,offset});}
  offset+=line.length+1;
 }return result;
}
export interface MdNode{type:string;value?:string;children?:MdNode[];url?:string;alt?:string;depth?:number;data?:{hProperties?:{id?:string}}}
function nodeText(node:MdNode):string{return node.value??node.children?.map(nodeText).join('')??'';}
export function remarkKnowledge(){return(tree:MdNode)=>{
 const seen=new Map<string,number>();
 const walk=(node:MdNode)=>{
  if(['code','inlineCode','html','link','image'].includes(node.type))return;
  if(node.type==='heading'){const slug=headingSlug(nodeText(node)),n=seen.get(slug)??0;seen.set(slug,n+1);node.data={...node.data,hProperties:{...node.data?.hProperties,id:slug+(n?`-${n}`:'')}};}
  if(!node.children)return;const next:MdNode[]=[];
  for(const child of node.children){if(child.type!=='text'||!child.value){walk(child);next.push(child);continue;}let at=0;const pattern=/(!?)\[\[([^\]\n]+)\]\]/g;for(const m of child.value.matchAll(pattern)){
   if(m.index!>at)next.push({type:'text',value:child.value.slice(at,m.index)});const [target,...alias]=m[2].split('|');const label=alias.join('|')||target;
   next.push(m[1]?{type:'image',url:'yingliu-asset:'+encodeURIComponent(target),alt:label}:{type:'link',url:'yingliu-wiki:'+encodeURIComponent(target),children:[{type:'text',value:label}]});at=m.index!+m[0].length;
  }if(at<child.value.length)next.push({...child,value:child.value.slice(at)});
  }node.children=next;
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
