import {assertWorkspaceLayout,defaultWorkspace,validateWorkspace} from './defaults.ts';
import type {WorkspaceLayout,WorkspacePreset,WorkspaceSettings} from './types.ts';

export function BUILTIN_WORKSPACE_PRESETS():WorkspacePreset[]{
 const base=defaultWorkspace().currentLayout;
 const learning=structuredClone(base);learning.library.view='cards';learning.library.showContinue=true;learning.player.layout='stacked';
 const focus=structuredClone(base);focus.player.layout='focus';focus.player.panelCollapsed=true;focus.library.view='list';focus.library.categoryCollapsed=true;
 const batch=structuredClone(base);batch.library.view='table';batch.library.columns=['title','category','items','duration','progress','availability','actions'];batch.player.panelCollapsed=true;
 const knowledge=structuredClone(base);knowledge.knowledge.treeCollapsed=false;knowledge.knowledge.infoCollapsed=false;knowledge.knowledge.treeWidth=280;knowledge.knowledge.infoWidth=320;
 const theater=structuredClone(base);theater.player.layout='theater';
 return [
  {id:'builtin.default',name:'默认',layout:base},
  {id:'builtin.learning',name:'视频学习',layout:learning},
  {id:'builtin.focus',name:'专注播放',layout:focus},
  {id:'builtin.batch',name:'批量管理',layout:batch},
  {id:'builtin.knowledge',name:'知识整理',layout:knowledge},
  {id:'builtin.theater',name:'剧院模式',layout:theater},
 ].map(p=>({...p,layout:structuredClone(p.layout)}));
}

function safeName(name:string):string{
 const trimmed=name.trim();
 if(!trimmed||trimmed.length>80||/[\u0000-\u001f\u007f-\u009f]/u.test(trimmed))throw new Error('预设名称须为1～80字且不能包含控制字符');
 return trimmed;
}
function safeId(id:string):void{
 if(!id||id.length>100||id.startsWith('builtin.')||/[\u0000-\u001f\u007f-\u009f]/u.test(id))throw new Error('预设 ID 无效或为内置保留 ID');
}
function custom(settings:WorkspaceSettings):WorkspaceSettings{return validateWorkspace(settings);}

export function createWorkspacePreset(settings:WorkspaceSettings,preset:WorkspacePreset):WorkspaceSettings{
 const current=custom(settings);safeId(preset.id);const next={...current,presets:[...current.presets,{id:preset.id,name:safeName(preset.name),layout:assertWorkspaceLayout(preset.layout)}]};
 if(next.presets.length>50)throw new Error('最多保存50个自定义预设');
 return validateWorkspace(next);
}
export function updateWorkspacePreset(settings:WorkspaceSettings,id:string,layout:WorkspaceLayout):WorkspaceSettings{
 const current=custom(settings);safeId(id);let found=false;
 const presets=current.presets.map(p=>p.id===id?(found=true,{...p,layout:assertWorkspaceLayout(layout)}):p);
 if(!found)throw new Error('自定义预设不存在');
 return validateWorkspace({...current,presets});
}
export function renameWorkspacePreset(settings:WorkspaceSettings,id:string,name:string):WorkspaceSettings{
 const current=custom(settings);safeId(id);let found=false;
 const presets=current.presets.map(p=>p.id===id?(found=true,{...p,name:safeName(name)}):p);
 if(!found)throw new Error('自定义预设不存在');
 return validateWorkspace({...current,presets});
}
export function deleteWorkspacePreset(settings:WorkspaceSettings,id:string):WorkspaceSettings{
 const current=custom(settings);safeId(id);const presets=current.presets.filter(p=>p.id!==id);
 if(presets.length===current.presets.length)throw new Error('自定义预设不存在');
 return validateWorkspace({...current,presets,activePreset:current.activePreset===id?null:current.activePreset});
}

export function applyWorkspacePreset(settings:WorkspaceSettings,id:string):WorkspaceSettings{
 const current=validateWorkspace(settings);
 const preset=[...BUILTIN_WORKSPACE_PRESETS(),...current.presets].find(p=>p.id===id);
 if(!preset)throw new Error('布局预设不存在');
 return validateWorkspace({...current,currentLayout:structuredClone(preset.layout),activePreset:id});
}

export function parseWorkspaceJson(text:string):WorkspaceSettings{
 if(new TextEncoder().encode(text).length>128*1024)throw new Error('工作区配置超过128 KiB');
 let parsed:unknown;try{parsed=JSON.parse(text);}catch{throw new Error('工作区配置不是有效 JSON');}
 return validateWorkspace(parsed);
}
export function serializeWorkspaceJson(settings:WorkspaceSettings):string{
 return JSON.stringify(validateWorkspace(settings),null,2);
}
