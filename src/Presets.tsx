import { useState } from 'react';
import { Plus, SlidersHorizontal, Trash2 } from 'lucide-react';
import { applyPreset } from './lib';
import type { DownloadPreset, MediaKind, VideoMode } from './types';

export const emptyPreset: DownloadPreset = {name:'',kind:'video',videoMode:'compatible',maxHeight:0,subtitleLanguages:[],filenameTemplate:'',byAuthor:false};

export function NamingFields({template, byAuthor, setTemplate, setByAuthor}: {template:string; byAuthor:boolean; setTemplate:(value:string)=>void; setByAuthor:(value:boolean)=>void}) {
  return <div className="naming-fields"><label>文件名模板<input value={template} maxLength={4096} onChange={event=>setTemplate(event.target.value)} placeholder="留空使用原有命名方式" /></label><p className="help">可用变量：{'{标题}、{作者}、{日期}、{播放列表序号}、{视频ID}'}。扩展名自动添加；文件名最长 180 字符。</p><label className="check-label"><input type="checkbox" checked={byAuthor} onChange={event=>setByAuthor(event.target.checked)} />按作者分目录</label></div>;
}

export function PresetSettings({presets, save}: {presets:DownloadPreset[]; save:(presets:DownloadPreset[])=>Promise<boolean>}) {
  const [editing,setEditing]=useState<number|null>(null);
  const [draft,setDraft]=useState<DownloadPreset|null>(null);
  const [saving,setSaving]=useState(false);
  const change=<K extends keyof DownloadPreset>(key:K,value:DownloadPreset[K])=>setDraft(previous=>previous ? {...previous,[key]:value} : previous);
  const persist=async(next:DownloadPreset[])=>{setSaving(true);try {if(await save(next)) {setDraft(null);setEditing(null);}} finally {setSaving(false);}};
  return <section className="settings-card"><div className="section-icon"><SlidersHorizontal size={20} /></div><div className="settings-section">
    <div className="setting-line top"><div><h2>下载预设</h2><p className="section-description">已保存 {presets.length} / 20 个；修改预设不影响已加入队列的任务</p></div><button className="secondary" disabled={saving || presets.length>=20} onClick={()=>{setEditing(null);setDraft(applyPreset(emptyPreset));}}><Plus size={16} />新建预设</button></div>
    <div className="preset-list">{presets.map((preset,index)=><div className="preset-item" key={preset.name}><div><strong>{preset.name}</strong><p className="help">{{video:'视频',audio:'音频',subtitles:'字幕'}[preset.kind]} · {preset.maxHeight ? `${preset.maxHeight}p` : '最佳可用画质'} · {preset.byAuthor ? '按作者分目录' : '保存到下载目录'}</p></div><button className="text-button" aria-label={`编辑预设：${preset.name}`} disabled={saving} onClick={()=>{setEditing(index);setDraft(applyPreset(preset));}}>编辑</button><button className="icon-button" aria-label={`删除预设：${preset.name}`} disabled={saving} onClick={()=>{void persist(presets.filter((_,i)=>i!==index));}}><Trash2 size={17}/></button></div>)}</div>
    {draft && <div className="preset-editor">
      <div className="option-grid"><label>预设名称<input value={draft.name} maxLength={40} onChange={event=>change('name',event.target.value)} /></label><label>下载类型<select value={draft.kind} onChange={event=>change('kind',event.target.value as MediaKind)}><option value="video">视频</option><option value="audio">音频 · MP3</option><option value="subtitles">字幕 · SRT</option></select></label></div>
      <div className="option-grid"><label>格式策略<select value={draft.videoMode} disabled={draft.kind!=='video'} onChange={event=>change('videoMode',event.target.value as VideoMode)}><option value="compatible">兼容优先 · MP4 / H.264 / AAC</option><option value="source">源站最高画质 · 保留源编码</option></select></label><label>最高画质<select value={draft.maxHeight} disabled={draft.kind!=='video'} onChange={event=>change('maxHeight',Number(event.target.value))}>{[...new Set([0,2160,1080,720,480,360,draft.maxHeight])].map(height=><option value={height} key={height}>{height ? `${height}p` : '最佳可用画质'}</option>)}</select></label></div>
      <label>字幕语言代码<input value={draft.subtitleLanguages.join(',')} onChange={event=>change('subtitleLanguages',event.target.value.split(','))} placeholder="zh-Hans,en；留空不附加字幕" /></label>
      <NamingFields template={draft.filenameTemplate} byAuthor={draft.byAuthor} setTemplate={value=>change('filenameTemplate',value)} setByAuthor={value=>change('byAuthor',value)} />
      <div className="preset-buttons"><button className="secondary" disabled={saving} onClick={()=>setDraft(null)}>放弃修改</button><button className="primary" disabled={saving || !draft.name.trim()} onClick={()=>{const preset={...draft,name:draft.name.trim(),subtitleLanguages:draft.subtitleLanguages.map(value=>value.trim()).filter(Boolean)};void persist(editing==null ? [...presets,preset] : presets.map((item,index)=>index===editing ? preset : item));}}>{saving?'正在保存…':'保存预设'}</button></div>
    </div>}
  </div></section>;
}
