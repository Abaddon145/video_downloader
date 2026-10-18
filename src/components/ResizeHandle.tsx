import {useRef} from 'react';
import {resizeKey,resizeValue} from './resize';
export function ResizeHandle({value,onChange,onCommit,min,max,orientation='vertical',label,reverse=false}:{value:number;onChange:(value:number)=>void;onCommit?:(value:number)=>void;min:number;max:number;orientation?:'vertical'|'horizontal';label:string;reverse?:boolean}){
 const drag=useRef<{start:number;value:number;latest:number}|null>(null);
 return <div role="separator" tabIndex={0} aria-label={label} aria-orientation={orientation} aria-valuemin={min} aria-valuemax={max} aria-valuenow={Math.round(value)} className={`resize-handle resize-${orientation}`}
 onPointerDown={e=>{if(e.button!==0)return;e.preventDefault();e.currentTarget.setPointerCapture(e.pointerId);drag.current={start:orientation==='vertical'?e.clientX:e.clientY,value,latest:value};}}
 onPointerMove={e=>{const d=drag.current;if(!d)return;d.latest=resizeValue(d.value,((orientation==='vertical'?e.clientX:e.clientY)-d.start)*(reverse?-1:1),min,max);onChange(d.latest);}}
 onPointerUp={e=>{if(!drag.current)return;onCommit?.(drag.current.latest);drag.current=null;if(e.currentTarget.hasPointerCapture(e.pointerId))e.currentTarget.releasePointerCapture(e.pointerId);}}
 onPointerCancel={()=>{if(drag.current)onCommit?.(drag.current.latest);drag.current=null;}}
 onKeyDown={e=>{const next=resizeKey(value,e.key,min,max,e.shiftKey?40:10,reverse);if(next===null)return;e.preventDefault();e.stopPropagation();onChange(next);onCommit?.(next);}}><span/></div>;
}
