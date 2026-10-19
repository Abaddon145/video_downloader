import {LearningDialog} from '../learning/Editors';
import {headings} from './lib';
export function NoteMindMap({title,text,jump,close}:{title:string;text:string;jump:(id:string)=>void;close:()=>void}){
 const all=headings(text),nodes=all.slice(0,80),width=1100,height=Math.max(260,nodes.length*62+50);
 const edges=nodes.map((h,i)=>{let p=i-1;while(p>=0&&nodes[p].depth>=h.depth)p--;return p;});
 return <LearningDialog title="笔记思维导图" close={close}><p className="help">{title} · 点击节点定位标题。{all.length>80?'为保持流畅，仅显示前80个标题。':''}</p>{nodes.length?<div className="note-mindmap"><svg role="img" aria-label={`${title}的标题结构`} width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
  {edges.map((p,i)=>{const x=nodes[i].depth*145,y=i*62+40,px=p<0?15:nodes[p].depth*145+128,py=p<0?30:p*62+40;return <path key={`e${i}`} d={`M${px},${py} H${x-12} V${y} H${x}`} fill="none" stroke="var(--border)" strokeWidth="2"/>;})}
  {nodes.map((h,i)=><g key={h.id} role="button" tabIndex={0} aria-label={`跳到 ${h.text}`} onClick={()=>{jump(h.id);close();}} onKeyDown={e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();jump(h.id);close();}}}><rect x={h.depth*145} y={i*62+19} width="128" height="42" rx="6" fill="var(--panel)" stroke="var(--cyan)"/><text x={h.depth*145+9} y={i*62+45} fill="var(--text)" fontSize="12">{h.text.length>9?h.text.slice(0,9)+'…':h.text}</text><title>{h.text}</title></g>)}
 </svg></div>:<p className="help">添加 Markdown 标题后即可生成结构导图。</p>}</LearningDialog>;
}
