import {useCallback,useLayoutEffect,useRef,useState} from 'react';
import type {KeyboardEvent,MouseEvent} from 'react';
import {createPortal} from 'react-dom';
import {clampMenu,firstEnabled,isEditableTarget,itemAtPath,nextEnabled} from './contextMenuLogic';
import type {ContextMenuItem} from './contextMenuLogic';
import './contextMenu.css';

export type {ContextMenuItem} from './contextMenuLogic';
export interface ContextMenuState<T>{id:number;target:T;x:number;y:number;returnFocus:HTMLElement}
type Trigger=MouseEvent<HTMLElement>|KeyboardEvent<HTMLElement>;

export function useContextMenu<T>(){
 const [menu,setMenu]=useState<ContextMenuState<T>|null>(null),current=useRef<ContextMenuState<T>|null>(null),serial=useRef(0);
 const close=useCallback(()=>{const focus=current.current?.returnFocus;current.current=null;setMenu(null);queueMicrotask(()=>{if(focus?.isConnected)focus.focus();});},[]);
 const open=useCallback((event:Trigger,target:T)=>{
  if('key' in event&&(event.key!=='F10'||!event.shiftKey))return;
  if(isEditableTarget(event.target as HTMLElement))return;
  event.preventDefault();event.stopPropagation();
  const element=event.currentTarget,rect=element.getBoundingClientRect();
  const next={id:++serial.current,target,x:'clientX' in event?event.clientX:rect.left,y:'clientY' in event?event.clientY:rect.bottom,returnFocus:element};
  current.current=next;setMenu(next);
 },[]);
 return {menu,open,close};
}

export function ContextMenu<T>({menu,items,onClose,onAction}:{menu:ContextMenuState<T>|null;items:ContextMenuItem[];onClose:()=>void;onAction:(item:ContextMenuItem,target:T)=>void|Promise<unknown>}){
 const root=useRef<HTMLDivElement>(null),buttons=useRef(new Map<string,HTMLButtonElement>()),current=useRef(menu);
 current.current=menu;
 const [position,setPosition]=useState({x:0,y:0}),[submenus,setSubmenus]=useState<Record<string,{x:number;y:number}>>({});
 const [active,setActive]=useState<number[]>([]),[opened,setOpened]=useState<number[]>([]),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const key=(path:number[])=>path.join('-');
 const isOpen=(path:number[])=>path.every((index,depth)=>opened[depth]===index);
 const atLevel=(path:number[])=>path.length?itemAtPath(items,path)?.children??[]:items;
 useLayoutEffect(()=>{
  if(!menu)return;
  setActive([firstEnabled(items)]);setOpened([]);setSubmenus({});setBusy(false);setError('');
  const box=root.current,panel=box?.querySelector<HTMLElement>('.context-menu-panel');
  if(box&&panel){setPosition(clampMenu(menu.x,menu.y,panel.offsetWidth,panel.offsetHeight,window.innerWidth,window.innerHeight));box.focus();}
 },[menu?.id]);
 useLayoutEffect(()=>{const item=buttons.current.get(key(active));item?.scrollIntoView({block:'nearest'});},[active]);
 useLayoutEffect(()=>{if(!menu)return;const dismiss=(event:PointerEvent)=>{if(!root.current?.contains(event.target as Node))onClose();};document.addEventListener('pointerdown',dismiss);return()=>document.removeEventListener('pointerdown',dismiss);},[menu?.id,onClose]);
 if(!menu)return null;
 const openChild=(path:number[],focusChild:boolean)=>{
  const item=itemAtPath(items,path),children=item?.children??[],button=buttons.current.get(key(path));
  const first=firstEnabled(children);if(!button||first<0)return;
  const rect=button.getBoundingClientRect(),width=Math.min(224,window.innerWidth-16),height=Math.min(window.innerHeight-16,children.length*38+16);
  const x=rect.right+width+2<=window.innerWidth-8?rect.right+2:rect.left-width-2;
  setSubmenus(previous=>({...previous,[key(path)]:clampMenu(x,rect.top,width,height,window.innerWidth,window.innerHeight)}));
  setOpened(path);
  if(focusChild)setActive([...path,first]);
 };
 const execute=async(item:ContextMenuItem)=>{
  if(item.disabled||busy)return;
  setBusy(true);setError('');
  try {await onAction(item,menu.target);if(current.current?.id===menu.id)onClose();}
  catch(e){if(current.current?.id===menu.id)setError(String(e));}
  finally {if(current.current?.id===menu.id)setBusy(false);}
 };
 const onKeyDown=(event:KeyboardEvent)=>{
  const level=active.slice(0,-1),levelItems=atLevel(level),index=active.at(-1)??-1;
  if(event.key==='Tab'){event.preventDefault();event.stopPropagation();onClose();return;}
  if(event.key==='Escape'){event.preventDefault();event.stopPropagation();onClose();return;}
  if(event.key==='ArrowDown'||event.key==='ArrowUp'||event.key==='Home'||event.key==='End'){
   event.preventDefault();event.stopPropagation();
   const next=event.key==='Home'?firstEnabled(levelItems):event.key==='End'?nextEnabled(levelItems,0,-1):nextEnabled(levelItems,index,event.key==='ArrowDown'?1:-1);
   if(next>=0){setActive([...level,next]);setOpened(level);}return;
  }
  if(event.key==='ArrowLeft'){event.preventDefault();event.stopPropagation();if(active.length>1){setActive(level);setOpened(level.slice(0,-1));}return;}
  if(event.key==='ArrowRight'||event.key==='Enter'||event.key===' '){event.preventDefault();event.stopPropagation();const item=itemAtPath(items,active);if(!item||item.disabled)return;if(item.children?.length)openChild(active,true);else if(event.key!=='ArrowRight')void execute(item);}
 };
 const render=(levelItems:ContextMenuItem[],prefix:number[],point:{x:number;y:number})=><div className="context-menu-panel" role={prefix.length?'menu':'group'} aria-label={prefix.length?'子菜单':undefined} style={{left:point.x,top:point.y}} onScroll={event=>{if(prefix.length===0&&event.target===event.currentTarget)setOpened([]);}}>
  {levelItems.map((item,index)=>{const path=[...prefix,index],id=`context-menu-${menu.id}-${key(path)}`,children=item.children?.length?item.children:null,expanded=!!children&&isOpen(path);
   return <div className="context-menu-entry" role="none" key={`${item.id}-${index}`}>
    {index>0&&item.group!==levelItems[index-1].group&&<div role="separator" className="context-menu-separator"/>}
    <button id={id} ref={node=>{if(node)buttons.current.set(key(path),node);else buttons.current.delete(key(path));}} type="button" role="menuitem" tabIndex={-1} disabled={item.disabled||busy} aria-haspopup={children?'menu':undefined} aria-expanded={children?expanded:undefined} data-active={key(active)===key(path)} data-danger={item.danger||undefined} title={item.availabilityReason||item.label}
     onMouseEnter={()=>{if(item.disabled)return;setActive(path);if(children)openChild(path,false);else setOpened(prefix);}}
     onClick={()=>{if(children)openChild(path,true);else void execute(item);}}>{item.icon&&<span aria-hidden="true">{item.icon}</span>}<span>{item.label}{item.disabled&&item.availabilityReason&&<small className="context-menu-reason">{item.availabilityReason}</small>}</span>{item.shortcut&&<kbd>{item.shortcut}</kbd>}{children&&<span aria-hidden="true" className="context-menu-arrow">›</span>}</button>
    {expanded&&children&&render(children,path,submenus[key(path)]??point)}
   </div>;
  })}
  {prefix.length===0&&(busy||error)&&<div className={error?'context-menu-error':'context-menu-status'} role={error?'alert':'status'}>{error||'正在执行…'}</div>}
 </div>;
 return createPortal(<div ref={root} className="context-menu-layer" data-native-overlay="true" role="menu" aria-label="快捷操作" aria-activedescendant={active[0]>=0?`context-menu-${menu.id}-${key(active)}`:undefined} tabIndex={-1} onKeyDown={onKeyDown} onContextMenu={event=>{event.preventDefault();event.stopPropagation();}}>
  {render(items,[],position)}
 </div>,document.body);
}
