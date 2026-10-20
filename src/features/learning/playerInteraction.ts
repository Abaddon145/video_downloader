export type PlayerAspect='original'|'16:9'|'4:3'|'fill';
export type TimelineMarker={timeMs:number;title:string;kind:'chapter'|'note'};
export function effectivePlayerLayout(layout:string,width:number,panelSize:number){
 return layout==='standard'&&(!Number.isFinite(width)||width<Math.max(760,panelSize+510))?'stacked':layout;
}
export function hoverPosition(clientX:number,left:number,width:number,duration:number):number|null {
 if(![clientX,left,width,duration].every(Number.isFinite)||width<=0||duration<=0)return null;
 return Math.min(1,Math.max(0,(clientX-left)/width))*duration;
}
export function timelineMarkers(markers:TimelineMarker[],duration:number){
 return markers.slice(0,1000).filter(m=>Number.isSafeInteger(m.timeMs)&&m.timeMs>=0&&m.timeMs<=duration*1000).slice(0,250).sort((a,b)=>a.timeMs-b.timeMs);
}
export const canHideControls=(playing:boolean,pointerInside:boolean,focusInside:boolean,overlayOpen:boolean)=>playing&&!pointerInside&&!focusInside&&!overlayOpen;
