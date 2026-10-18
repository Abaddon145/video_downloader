import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {PanelsTopLeft} from 'lucide-react';
import {useWorkspace} from './WorkspaceProvider';
import {defaultWorkspace,validateWorkspace} from './defaults';
import {BUILTIN_WORKSPACE_PRESETS,createWorkspacePreset,updateWorkspacePreset,renameWorkspacePreset,deleteWorkspacePreset} from './presets';
import type {WorkspaceAppearance,WorkspaceSettings} from './types';

export function WorkspaceSettingsPanel(){
 const {workspace,setAppearance,setLayout,updateWorkspace}=useWorkspace(),a=workspace.appearance,l=workspace.currentLayout;
 const [selected,setSelected]=useState('builtin.default'),[name,setName]=useState(''),[error,setError]=useState(''),[notice,setNotice]=useState(''),[busy,setBusy]=useState(false);
 const presets=[...BUILTIN_WORKSPACE_PRESETS(),...workspace.presets],custom=workspace.presets.some(p=>p.id===selected);
 const change=<K extends keyof WorkspaceAppearance>(key:K,value:WorkspaceAppearance[K])=>setAppearance(old=>({...old,[key]:value}));
 const run=(action:()=>void)=>{try{setError('');setNotice('');action();}catch(e){setError(String(e));}};
 const native=async(exporting:boolean)=>{setBusy(true);setError('');setNotice('');try{
  if(exporting){const done=await invoke<boolean>('workspace_export',{settings:validateWorkspace(workspace)});if(done)setNotice('布局配置已导出，仅包含外观、布局和预设。');}
  else {const imported=await invoke<WorkspaceSettings|null>('workspace_import');if(imported){const checked=validateWorkspace(imported);if(window.confirm('导入将替换当前外观、布局和自定义预设。是否继续？')){updateWorkspace(()=>checked);setSelected('builtin.default');setName('');setNotice('布局配置已导入。');}}}
 }catch(e){setError(String(e));}finally{setBusy(false);}};
 return <section className="settings-card workspace-settings"><div className="section-icon"><PanelsTopLeft size={20}/></div><div className="settings-section"><h2>外观与工作区</h2><p className="section-description">更改立即生效并保存在本机，适用于下载、学习和知识整理。</p>
  <div className="workspace-preferences-grid">
   <label>主题<select aria-label="主题" value={a.theme} onChange={e=>change('theme',e.target.value as typeof a.theme)}><option value="dark">深色</option><option value="light">浅色</option><option value="system">跟随系统</option></select></label>
   <label>强调色<input aria-label="强调色" type="color" value={a.accent} onChange={e=>change('accent',e.target.value)}/></label>
   <label>字体大小 <output>{a.fontSize}px</output><input aria-label="字体大小" type="range" min="12" max="20" step="1" value={a.fontSize} onChange={e=>change('fontSize',Number(e.target.value))}/></label>
   <label>信息密度<select aria-label="信息密度" value={a.density} onChange={e=>change('density',e.target.value as typeof a.density)}><option value="comfortable">舒适</option><option value="standard">标准</option><option value="compact">紧凑</option></select></label>
   <label>侧边栏宽度 <output>{a.sidebarWidth}px</output><input aria-label="侧边栏宽度" type="range" min="160" max="320" value={a.sidebarWidth} onChange={e=>change('sidebarWidth',Number(e.target.value))}/></label>
   <label>动画<select aria-label="动画" value={a.motion} onChange={e=>change('motion',e.target.value as typeof a.motion)}><option value="system">跟随系统</option><option value="reduced">减少动画</option><option value="full">完整动画</option></select></label>
  </div>
  <details className="workspace-panel-controls"><summary>卡片、面板与折叠状态</summary><div className="workspace-preferences-grid">
   <label>学习卡片尺寸 <output>{l.library.cardSize}px</output><input aria-label="学习卡片尺寸" type="range" min="160" max="480" value={l.library.cardSize} onChange={e=>setLayout(old=>({...old,library:{...old.library,cardSize:Number(e.target.value)}}))}/></label>
   <label>学习面板尺寸 <output>{l.player.panelSize}px</output><input aria-label="学习面板尺寸" type="range" min="220" max="720" value={l.player.panelSize} onChange={e=>setLayout(old=>({...old,player:{...old.player,panelSize:Number(e.target.value)}}))}/></label>
   <label className="workspace-check"><input type="checkbox" checked={l.library.categoryCollapsed} onChange={e=>setLayout(old=>({...old,library:{...old.library,categoryCollapsed:e.target.checked}}))}/>折叠学习分类</label>
   <label className="workspace-check"><input type="checkbox" checked={l.player.panelCollapsed} onChange={e=>setLayout(old=>({...old,player:{...old.player,panelCollapsed:e.target.checked}}))}/>折叠学习面板</label>
   <label className="workspace-check"><input type="checkbox" checked={l.knowledge.treeCollapsed} onChange={e=>setLayout(old=>({...old,knowledge:{...old.knowledge,treeCollapsed:e.target.checked}}))}/>折叠知识文件树</label>
   <label className="workspace-check"><input type="checkbox" checked={l.knowledge.infoCollapsed} onChange={e=>setLayout(old=>({...old,knowledge:{...old.knowledge,infoCollapsed:e.target.checked}}))}/>折叠知识信息栏</label>
  </div></details>
  <div className="workspace-preset-controls"><h3>布局预设</h3><p className="help">预设保存面板布局，外观设置保持独立。内置预设可应用，自定义预设可更新。</p>
   <div className="workspace-preferences-grid"><label>布局预设<select aria-label="布局预设" value={selected} onChange={e=>{setSelected(e.target.value);setName(workspace.presets.find(p=>p.id===e.target.value)?.name??'');}}>{presets.map(p=><option key={p.id} value={p.id}>{p.name}{p.id.startsWith('builtin.')?' · 内置':''}</option>)}</select></label><label>预设名称<input aria-label="预设名称" value={name} maxLength={80} onChange={e=>setName(e.target.value)} placeholder="为当前布局命名"/></label></div>
   <div className="workspace-action-row">
    <button className="secondary" onClick={()=>run(()=>{const p=presets.find(p=>p.id===selected);if(p)setLayout(()=>structuredClone(p.layout));setNotice('已应用布局预设。');})}>应用预设</button>
    <button className="secondary" disabled={!name.trim()} onClick={()=>run(()=>{const id=`custom-${crypto.randomUUID()}`;updateWorkspace(old=>createWorkspacePreset(old,{id,name,layout:old.currentLayout}));setSelected(id);setNotice('自定义预设已新建。');})}>新建预设</button>
    <button className="secondary" disabled={!custom} onClick={()=>run(()=>{updateWorkspace(old=>updateWorkspacePreset(old,selected,old.currentLayout));setNotice('当前布局已保存到预设。');})}>保存当前布局</button>
    <button className="secondary" disabled={!custom||!name.trim()} onClick={()=>run(()=>{updateWorkspace(old=>renameWorkspacePreset(old,selected,name));setNotice('预设已重命名。');})}>重命名预设</button>
    <button className="secondary" disabled={!custom} onClick={()=>run(()=>{if(window.confirm('删除这个自定义预设？当前布局保持不变。')){updateWorkspace(old=>deleteWorkspacePreset(old,selected));setSelected('builtin.default');setName('');setNotice('预设已删除。');}})}>删除预设</button>
   </div>
  </div>
  <div className="workspace-action-row"><button className="secondary" disabled={busy} onClick={()=>{void native(false);}}>导入布局配置</button><button className="secondary" disabled={busy} onClick={()=>{void native(true);}}>导出布局配置</button><button className="text-button" onClick={()=>run(()=>{if(window.confirm('恢复默认外观和布局？自定义预设会保留。')){updateWorkspace(old=>({...defaultWorkspace(),presets:old.presets}));setNotice('已恢复默认工作区，自定义预设已保留。');}})}>恢复默认工作区</button></div>
  <p className="help">配置仅包含外观、布局和预设，不包含 API Key、Cookie 或账号信息。</p>{error&&<p role="alert" className="error-text">{error}</p>}{notice&&<p role="status" className="help">{notice}</p>}
 </div></section>;
}

