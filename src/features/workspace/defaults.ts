import type {WorkspaceSettings, WorkspaceLayout} from './types.ts';
export const workspaceColumns=['title','category','items','duration','progress','aiStatus','availability','lastPlayed','updated','actions'];
export function defaultWorkspace():WorkspaceSettings {return {version:1,appearance:{theme:'dark',accent:'#38BDF8',fontSize:14,density:'standard',sidebarWidth:194,motion:'system'},currentLayout:{library:{view:'cards',categoryWidth:224,categoryCollapsed:false,cardSize:260,cardsPerRow:null,coverRatio:'16:9',showProgress:true,showAiStatus:true,showContinue:true,sort:'manual',columns:[...workspaceColumns],columnWidths:{}},player:{layout:'standard',panelSize:360,panelCollapsed:false},knowledge:{treeWidth:240,infoWidth:280,treeCollapsed:false,infoCollapsed:false}},presets:[]};}
const between=(n:unknown,min:number,max:number)=>typeof n==='number'&&Number.isFinite(n)&&n>=min&&n<=max;
function layoutValid(l:WorkspaceLayout):boolean {
 const {library:b,player:p,knowledge:k}=l;
 return ['cards','list','table'].includes(b.view)&&between(b.categoryWidth,160,480)&&between(b.cardSize,160,480)&&(b.cardsPerRow===null||between(b.cardsPerRow,1,8)&&Number.isInteger(b.cardsPerRow))&&['16:9','4:3','square'].includes(b.coverRatio)&&['manual','title','updated','lastPlayed','progress','duration'].includes(b.sort)&&Array.isArray(b.columns)&&b.columns.includes('title')&&b.columns.includes('actions')&&b.columns.length<=workspaceColumns.length&&new Set(b.columns).size===b.columns.length&&b.columns.every(c=>workspaceColumns.includes(c))&&Object.entries(b.columnWidths).every(([c,w])=>workspaceColumns.includes(c)&&between(w,64,800))&&['standard','stacked','theater','focus'].includes(p.layout)&&between(p.panelSize,220,720)&&between(k.treeWidth,160,480)&&between(k.infoWidth,200,600);
}
function shape(value:unknown,base:unknown):boolean {
 if(Array.isArray(base))return Array.isArray(value);
 if(base===null)return value===null||typeof value==='number';
 if(typeof base!=='object')return typeof value===typeof base;
 if(!value||typeof value!=='object'||Array.isArray(value))return false;
 const expected=base as Record<string,unknown>,actual=value as Record<string,unknown>;
 if(!Object.keys(expected).length)return Object.values(actual).every(v=>typeof v==='number');
 return Object.keys(actual).length===Object.keys(expected).length&&Object.entries(expected).every(([k,v])=>shape(actual[k],v));
}
export function readWorkspace(value:unknown):WorkspaceSettings {
 const fallback=defaultWorkspace();
 try {
  if(!shape(value,fallback))return fallback;
  const s=value as WorkspaceSettings,a=s.appearance;
  if(s.version!==1||!['dark','light','system'].includes(a.theme)||!/^#[0-9a-f]{6}$/i.test(a.accent)||!between(a.fontSize,12,20)||!['comfortable','standard','compact'].includes(a.density)||!between(a.sidebarWidth,160,320)||!['system','reduced','full'].includes(a.motion)||!layoutValid(s.currentLayout)||s.presets.length>50)return fallback;
  const ids=new Set<string>();
  for(const p of s.presets){if(!shape(p,{id:'',name:'',layout:fallback.currentLayout})||!p.id||p.id.length>100||!p.name.trim()||p.name.length>80||ids.has(p.id)||!layoutValid(p.layout))return fallback;ids.add(p.id);}
  return structuredClone(s);
 }catch{return fallback;}
}
export function saveWorkspaceQueue(save:(value:WorkspaceSettings)=>Promise<unknown>,onError:(error:unknown)=>void=()=>{}) {
 let pending:WorkspaceSettings|undefined,running=false;
 const flush=async()=>{running=true;while(pending){const next=pending;pending=undefined;try{await save(next);}catch(error){onError(error);}}running=false;};
 return (value:WorkspaceSettings)=>{pending=value;if(!running)void flush();};
}
