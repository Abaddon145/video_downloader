export interface ContextMenuItem {
 id:string;
 label:string;
 disabled?:boolean;
 danger?:boolean;
 group?:string;
 children?:ContextMenuItem[];
}

export function firstEnabled(items:ContextMenuItem[]){return items.findIndex(item=>!item.disabled);}
export function nextEnabled(items:ContextMenuItem[],from:number,step:1|-1,wrap=true){
 if(!items.length)return -1;
 let index=from;
 for(let count=0;count<items.length;count++){
  const candidate=index+step;
  if(!wrap&&(candidate<0||candidate>=items.length))return index;
  index=(candidate+items.length)%items.length;
  if(!items[index].disabled)return index;
 }
 return from;
}
export function itemAtPath(items:ContextMenuItem[],path:number[]):ContextMenuItem|undefined {
 let item:ContextMenuItem|undefined;
 for(const index of path){item=items[index];if(!item)return;items=item.children??[];}
 return item;
}
export function clampMenu(x:number,y:number,width:number,height:number,viewportWidth:number,viewportHeight:number){
 return {x:Math.max(8,Math.min(x,viewportWidth-width-8)),y:Math.max(8,Math.min(y,viewportHeight-height-8))};
}
export function isEditableTarget(target:{closest?:(selector:string)=>unknown}|null){
 return !!target?.closest?.('input,textarea,select,[contenteditable],[role="textbox"]');
}
