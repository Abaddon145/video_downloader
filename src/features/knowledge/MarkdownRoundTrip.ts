import {unified} from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';

export type RichNode={type:string;text?:string;marks?:{type:string}[];attrs?:{level?:number};content?:RichNode[]};
export type RichTextResult={mode:'rich';value:string;doc:RichNode}|{mode:'source';value:string;reason:string};
type MdNode={type:string;value?:string;depth?:number;children?:MdNode[];position?:{start:{offset?:number};end:{offset?:number}}};
const parser=unified().use(remarkParse).use(remarkGfm);
const special=/\uFEFF|\r|\[\[|\]\]|!\[|%%|==|::|(?:^|\s)#[\p{L}\p{N}]|\^[\p{L}\p{N}_-]+(?:\s|$)|[<>|$]/u;

function inline(nodes:MdNode[]|undefined,marks:('bold'|'italic')[]=[]):RichNode[]|null{
 const result:RichNode[]=[];
 for(const node of nodes??[]){
  if(node.type==='text'&&node.value!==undefined&&!node.value.includes('\n'))result.push({type:'text',text:node.value,...(marks.length?{marks:[...new Set(marks)].sort().map(type=>({type}))}:{})});
  else if(node.type==='strong'||node.type==='emphasis'){
   const nested=inline(node.children,[...marks,node.type==='strong'?'bold':'italic']);if(!nested)return null;result.push(...nested);
  }else return null;
 }
 return result;
}

function escapeText(value:string):string{
 return value.replace(/([\\`*_{}\[\]<>!|$^])/g,'\\$1').replace(/(^|\n)([#>+\-]|\d+\.)/g,'$1\\$2');
}

function renderInline(nodes:RichNode[]|undefined):string|null{
 let result='';
 for(const node of nodes??[]){
  if(node.type!=='text'||typeof node.text!=='string'||node.content||node.attrs||node.text.includes('\n'))return null;
  const marks=node.marks??[];
  if(marks.length>2||new Set(marks.map(m=>m.type)).size!==marks.length||marks.some(mark=>mark.type!=='bold'&&mark.type!=='italic'))return null;
  const value=escapeText(node.text),marker=(marks.some(m=>m.type==='bold')?'**':'')+(marks.some(m=>m.type==='italic')?'*':'');
  result+=marker+value+marker;
 }
 return result;
}

function render(doc:RichNode):string|null{
 if(doc.type!=='doc'||!Array.isArray(doc.content)||doc.text||doc.marks||doc.attrs)return null;
 const parts:string[]=[];
 for(const node of doc.content){
  if(node.type!=='paragraph'&&node.type!=='heading'||node.text||node.marks)return null;
  const body=renderInline(node.content);if(body===null)return null;
  if(node.type==='heading'){
   const level=node.attrs?.level;if(typeof level!=='number'||!Number.isInteger(level)||level<1||level>6||!body)return null;
   parts.push('#'.repeat(level)+' '+body);
  }else if(node.attrs)return null;else parts.push(body);
 }
 return parts.join('\n\n');
}

export function toRichText(value:string):RichTextResult{
 const fallback=(reason:string):RichTextResult=>({mode:'source',value,reason});
 if(special.test(value))return fallback('包含源码模式保留的 Markdown 或 Obsidian 语法');
 if(!value)return {mode:'rich',value,doc:{type:'doc',content:[{type:'paragraph'}]}};
 const tree=parser.parse(value) as MdNode;
 const content:RichNode[]=[],source=tree.children??[];
 for(const node of source){
  if(node.type!=='paragraph'&&node.type!=='heading'||node.type==='heading'&&(!node.depth||node.depth>6))return fallback('包含富文本暂不支持的 Markdown 结构');
  const children=inline(node.children);if(!children)return fallback('包含富文本暂不支持的行内语法');
  content.push({type:node.type,...(node.type==='heading'?{attrs:{level:node.depth}}:{}),content:children});
 }
 const doc:RichNode={type:'doc',content};
 // ponytail: reject noncanonical source instead of trying to preserve every Markdown spelling.
 let end=0;
 for(let i=0;i<source.length;i++){
  const start=source[i].position?.start.offset,stop=source[i].position?.end.offset;
  if(start===undefined||stop===undefined||!/^\n{0,2}$/.test(value.slice(end,start))||value.slice(start,stop)!==render({type:'doc',content:[content[i]]}))return fallback('Markdown 排版写法需要源码模式保留');
  end=stop;
 }
 if(!/^\n?$/.test(value.slice(end)))return fallback('Markdown 排版写法需要源码模式保留');
 return {mode:'rich',value,doc};
}

export function fromRichText(doc:RichNode,initial:Extract<RichTextResult,{mode:'rich'}>):string|null{
 if(JSON.stringify(doc)===JSON.stringify(initial.doc))return initial.value;
 return render(doc);
}
