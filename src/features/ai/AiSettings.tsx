import {WhisperModels} from './WhisperModels';
import {AiUsage} from './AiUsage';
import {lazy,Suspense,useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Sparkles,ShieldCheck} from 'lucide-react';
import type {AiSettings as Settings,AiSettingsView} from '../../types/ai';
import type {AnalysisTemplate} from '../../types/ai';
const AiTemplateManager=lazy(()=>import('./AiTemplateManager'));
export function AiSettings({notify}:{notify:(message:string)=>void}) {
 const [templates,setTemplates]=useState<AnalysisTemplate[]|null>(null);
 const [view,setView]=useState<AiSettingsView|null>(null),[form,setForm]=useState<Settings|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{let alive=true;invoke<AiSettingsView>('ai_get_settings').then(v=>{if(alive){setView(v);setForm(v.settings);}}).catch(e=>{if(alive)setError(String(e));});return()=>{alive=false;};},[]);
 function change<K extends keyof Settings>(key:K,value:Settings[K]){setForm(s=>s?{...s,[key]:value}:s);}
 async function action(operation:()=>Promise<void>){setBusy(true);setError('');try{await operation();}catch(e){setError(String(e));}finally{setBusy(false);}}
 async function refresh(){setView(await invoke<AiSettingsView>('ai_get_settings'));}
 return <section className="settings-card"><div className="section-icon"><Sparkles size={20}/></div><div className="settings-section"><h2>AI视频分析</h2><p className="section-description">本地识别语音，由 DeepSeek 整理课程内容。原视频与完整音频留在电脑上。</p>
 {error&&<p className="error-text" role="alert">{error}</p>}
 {form&&view&&<><div className="setting-line"><div><strong>DeepSeek API密钥</strong><p className="help"><ShieldCheck size={13}/> {view.hasApiKey?'•••••••• · Windows加密保存':'尚未设置'} · 在Windows安全窗口录入</p></div><div className="task-actions"><button className="secondary" disabled={busy} onClick={()=>void action(async()=>{if(await invoke<boolean>('ai_set_deepseek_key')){await refresh();notify('API密钥已加密保存');}})}>{view.hasApiKey?'更换':'设置密钥'}</button><button className="secondary" disabled={busy||!view.hasApiKey} onClick={()=>void action(async()=>{await invoke('ai_clear_deepseek_key');await refresh();notify('API密钥已删除');})}>删除</button></div></div>
 <div className="setting-line"><span>连接与模型</span><button className="secondary" disabled={busy||!view.hasApiKey} onClick={()=>void action(async()=>{notify(await invoke<string>('ai_test_deepseek'));})}>{busy?'正在处理…':'测试连接'}</button></div><p className="help">测试连接读取模型列表，不发送视频内容。AI使用系统网络代理；下载代理仍由原设置控制。</p>
 <label className="field-label" htmlFor="ai-model">分析模型</label><select id="ai-model" value={form.model} disabled={busy} onChange={e=>change('model',e.target.value)}><option value="deepseek-flash">DeepSeek Flash · 文字与画面</option><option value="deepseek-v4-pro" disabled={form.allowImages}>DeepSeek Pro · 仅文字</option></select>
 <div className="setting-line"><label htmlFor="ai-mode">默认分析模式</label><select id="ai-mode" value={form.defaultMode} onChange={e=>change('defaultMode',e.target.value as Settings['defaultMode'])}><option value="quick">快速</option><option value="standard">标准</option><option value="deep">深度</option></select></div>
 <div className="setting-line"><label htmlFor="ai-whisper">本地语音模型</label><select id="ai-whisper" value={form.whisperModel} onChange={e=>change('whisperModel',e.target.value as Settings['whisperModel'])}><option value="base">Base · 快速</option><option value="small">Small · 更精确</option><option value="custom">导入的本地模型</option></select></div>
 {([{key:'allowTranscript',text:'允许发送字幕文本给DeepSeek'},{key:'allowImages',text:'允许发送精选关键帧给DeepSeek'},{key:'cleanTemporaryFiles',text:'分析完成自动清理临时音频'},{key:'autoGenerateNotes',text:'默认在分析完成后生成知识笔记'},{key:'autoAnalyzeDownloads',text:'下载完成后自动分析（可能产生API费用）'}] as const).map(({key,text})=><div className="setting-line" key={key}><label htmlFor={`ai-${key}`}>{text}</label><label className="switch"><input id={`ai-${key}`} type="checkbox" checked={form[key]} disabled={busy} onChange={e=>change(key,e.target.checked)}/><span/></label></div>)}
 <div className="setting-line"><label htmlFor="ai-note-text">允许 AI 整理时发送选中的笔记文本</label><label className="switch"><input id="ai-note-text" type="checkbox" checked={form.allowNoteText??false} disabled={busy} onChange={e=>change('allowNoteText',e.target.checked)}/><span/></label></div>
 <p className="help">笔记文本权限默认关闭，开启后仍需每次主动点击并确认；不会跟随笔记链接读取其他内容。知识笔记开关作为新分析的默认选择，每次开始前可单独更改。已开始任务保留当次选择，需要停止时可在课程面板取消分析。</p>
 {!form.allowTranscript&&!form.allowImages&&<p className="help">已禁止上传内容，仍可本地转录。AI分析至少需要允许字幕或关键帧中的一种。</p>}
 <button className="primary" disabled={busy} onClick={()=>void action(async()=>{const saved=await invoke<AiSettingsView>('ai_save_settings',{settings:form});setView(saved);setForm(saved.settings);notify('AI设置已保存');})}>保存AI设置</button></>}
 <div className="setting-line"><div><strong>AI用途模板</strong><p className="help">复制内置用途或建立自己的整理目标；不调用 AI、不产生 API 费用。</p></div><button type="button" className="secondary" disabled={busy} onClick={()=>void action(async()=>{const list=await invoke<AnalysisTemplate[]>('ai_list_templates');if(!Array.isArray(list)||!list.length)throw '用途模板列表无效，请重试';setTemplates(list);})}>管理用途模板</button></div>
 <details className="ai-tools"><summary>语音模型、用量与缓存</summary><WhisperModels/><AiUsage/></details>{templates&&<Suspense fallback={<p role="status">正在打开用途模板…</p>}><AiTemplateManager templates={templates} selectedId="general" close={()=>setTemplates(null)} onChanged={async()=>{setTemplates(await invoke<AnalysisTemplate[]>('ai_list_templates'));}}/></Suspense>}</div></section>;
}
