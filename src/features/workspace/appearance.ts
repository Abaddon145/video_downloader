import type {WorkspaceAppearance} from './types.ts';

const luminance=(hex:string)=>{const rgb=hex.slice(1).match(/../g)!.map(n=>parseInt(n,16)/255).map(n=>n<=.04045?n/12.92:((n+.055)/1.055)**2.4);return .2126*rgb[0]+.7152*rgb[1]+.0722*rgb[2];};
export function appearanceTokens(a:WorkspaceAppearance,systemDark:boolean,systemReduced:boolean){
 const theme=a.theme==='system'?(systemDark?'dark':'light'):a.theme;
 const onAccent=luminance(a.accent)>.179?'#071119':'#ffffff';
 // Decorative accents may be any color; text accents must remain readable on the selected surface.
 const surface=theme==='light'?'#ffffff':'#191d23',l=luminance(a.accent),b=luminance(surface);
 const ink=(Math.max(l,b)+.05)/(Math.min(l,b)+.05)>=4.5?a.accent:theme==='light'?'#075985':'#7dd3fc';
 return {theme,motion:a.motion==='system'?(systemReduced?'reduced':'full'):a.motion,vars:{
  '--workspace-accent':a.accent,'--workspace-accent-ink':ink,'--workspace-on-accent':onAccent,
  '--workspace-font-scale':String(a.fontSize/14),'--workspace-sidebar-width':`${a.sidebarWidth}px`,
  '--workspace-density-space':a.density==='compact'?'8px':a.density==='comfortable'?'20px':'14px',
  '--workspace-row-padding':a.density==='compact'?'8px':a.density==='comfortable'?'18px':'12px',
 }};
}
