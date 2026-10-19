import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Copy,Plus,Star,Trash2} from 'lucide-react';
import type {AnalysisTemplate} from '../../types/ai';
import {LearningDialog} from '../learning/Editors';
import {aiError} from './lib';
import {sectionsFromText,validateTemplateForm} from './templates';
import './templates.css';
const blank=():AnalysisTemplate=>({id:'',version:1,name:'',description:'',focus:'',sections:[{id:'section-1',title:'学习重点'}],detail:'balanced',extractParameters:false,extractTerms:true,exercises:false,timeReferences:true,markdownTemplate:'',builtin:false,favorite:false});
export default function AiTemplateManager({templates,selectedId,close,onChanged}:{templates:AnalysisTemplate[];selectedId:string;close:()=>void;onChanged:(selected?:AnalysisTemplate)=>Promise<void>}){
 const [draft,setDraft]=useState<AnalysisTemplate>(()=>structuredClone(templates.find(t=>t.id===selectedId)??templates[0]??blank()));
 const [sectionText,setSectionText]=useState(draft.sections.map(s=>s.title).join('\n'));
 const [busy,setBusy]=useState(false),[error,setError]=useState(''),[message,setMessage]=useState(''),[deleting,setDeleting]=useState(false),[versions,setVersions]=useState<AnalysisTemplate[]>([]);
 const editable=!draft.builtin;
 const choose=(template:AnalysisTemplate)=>{setDraft(structuredClone(template));setSectionText(template.sections.map(s=>s.title).join('\n'));setError('');setMessage('');};
 const change=<K extends keyof AnalysisTemplate>(key:K,value:AnalysisTemplate[K])=>setDraft(current=>({...current,[key]:value}));
 useEffect(()=>{let live=true;setVersions([]);if(draft.id)void invoke<AnalysisTemplate[]>('ai_template_versions',{id:draft.id}).then(v=>{if(live)setVersions(v);}).catch(e=>{if(live)setError(aiError(e));});return()=>{live=false;};},[draft.id,draft.version]);
 async function action(run:()=>Promise<void>){if(busy)return;setBusy(true);setError('');setMessage('');try{await run();}catch(e){setError(aiError(e));}finally{setBusy(false);}}
 async function save(){const template={...draft,sections:sectionsFromText(sectionText)},invalid=validateTemplateForm(template);if(invalid){setError(invalid);return;}
  await action(async()=>{const saved=await invoke<AnalysisTemplate>('ai_save_template',{template,expectedVersion:draft.id?draft.version:null});choose(saved);await onChanged(saved);setMessage(`已保存“${saved.name}”v${saved.version}，历史分析保留原模板。`);});
 }
 return <><LearningDialog title="AI用途模板" className="ai-template-dialog" close={close} busy={busy}>
  <p className="help">用途决定整理哪些知识；快速、标准和深度仍单独控制分析范围。官方模板可复制后修改。</p>
  <div className="ai-template-manager"><nav aria-label="用途模板列表">{templates.map(t=><button type="button" key={t.id} disabled={busy} className={draft.id===t.id?'active':''} aria-pressed={draft.id===t.id} onClick={()=>choose(t)}><span>{t.favorite&&'★ '}{t.name}</span><small>{t.builtin?'内置':'自定义'} · v{t.version}</small></button>)}<button type="button" className="secondary" disabled={busy} onClick={()=>choose(blank())}><Plus size={14}/>新建模板</button></nav>
   <form onSubmit={e=>{e.preventDefault();void save();}}><div className="ai-template-manager-actions">
    <button type="button" className="secondary" disabled={busy||!draft.id} onClick={()=>void action(async()=>{const copied=await invoke<AnalysisTemplate>('ai_copy_template',{id:draft.id,version:draft.version});choose(copied);await onChanged(copied);setMessage('已创建独立副本，可以修改。');})}><Copy size={14}/>复制模板</button>
    <button type="button" className="secondary" disabled={busy||!draft.id} aria-pressed={draft.favorite} onClick={()=>void action(async()=>{const saved=await invoke<AnalysisTemplate>('ai_set_template_favorite',{id:draft.id,favorite:!draft.favorite});setDraft(current=>({...current,favorite:saved.favorite}));await onChanged();setMessage(saved.favorite?'已收藏模板':'已取消收藏');})}><Star size={14} fill={draft.favorite?'currentColor':'none'}/>{draft.favorite?'已收藏':'收藏'}</button>
    {!draft.builtin&&draft.id&&<button type="button" className="text-button" disabled={busy} onClick={()=>setDeleting(true)}><Trash2 size={14}/>删除模板</button>}
   </div>
   <fieldset disabled={busy||!editable}><legend>{draft.builtin?'官方模板 · 只读':'自定义用途'}</legend>
    <label>模板名称<input aria-label="模板名称" value={draft.name} maxLength={80} required onChange={e=>change('name',e.target.value)}/></label>
    <label>模板说明<textarea aria-label="模板说明" value={draft.description} maxLength={1000} rows={2} onChange={e=>change('description',e.target.value)}/></label>
    <label>分析重点<textarea aria-label="分析重点" value={draft.focus} maxLength={4000} rows={4} required onChange={e=>change('focus',e.target.value)}/></label>
    <label>输出栏目 · 每行一个<textarea aria-label="输出栏目" value={sectionText} maxLength={1000} rows={4} required onChange={e=>setSectionText(e.target.value)}/></label>
    <label>内容详尽程度<select aria-label="内容详尽程度" value={draft.detail} onChange={e=>change('detail',e.target.value as AnalysisTemplate['detail'])}><option value="concise">简洁</option><option value="balanced">标准</option><option value="detailed">详尽</option></select></label>
    <div className="ai-template-flags">{([{key:'extractParameters',label:'提取关键参数'},{key:'extractTerms',label:'提取专业术语'},{key:'exercises',label:'整理练习题'},{key:'timeReferences',label:'显示时间引用'}]as const).map(({key,label})=><label className="check-label" key={key}><input type="checkbox" checked={draft[key]} onChange={e=>change(key,e.target.checked)}/>{label}</label>)}</div>
    <details><summary>知识笔记格式</summary><p className="help">留空使用内置格式。可使用 {'{{templateName}}、{{title}}、{{oneSentenceSummary}}、{{overview}}、{{chapters}}、{{keyPoints}}、{{sections}}、{{tutorialSteps}}、{{terms}}、{{actionItems}}、{{links}}'}。</p><textarea aria-label="知识笔记格式" className="ai-template-markdown" rows={6} maxLength={16000} value={draft.markdownTemplate} onChange={e=>change('markdownTemplate',e.target.value)}/></details>
   </fieldset>
   {!!versions.length&&<details className="ai-template-versions"><summary>版本记录 · {versions.length}版</summary>{versions.map(v=><article key={v.version}><strong>v{v.version} · {v.name}</strong><p>{v.description||v.focus}</p></article>)}</details>}
   {!draft.builtin&&<button type="submit" className="primary" disabled={busy}>{busy?'正在保存…':draft.id?'保存为新版本':'创建模板'}</button>}
   </form>
  </div>
  {error&&<p className="error-text" role="alert">{error}</p>}{message&&<p className="help" role="status">{message}</p>}<footer><button type="button" className="secondary" disabled={busy} onClick={close}>完成</button></footer>
 </LearningDialog>{deleting&&<LearningDialog title="删除自定义模板" close={()=>setDeleting(false)} busy={busy}><p>删除“{draft.name}”的模板列表记录？已有分析和其中的历史模板快照会保留。</p><footer><button type="button" className="secondary" disabled={busy} onClick={()=>setDeleting(false)}>取消</button><button type="button" className="danger-button" disabled={busy} onClick={()=>void action(async()=>{await invoke('ai_delete_template',{id:draft.id,expectedVersion:draft.version});await onChanged();setDeleting(false);choose(templates.find(t=>t.builtin)??blank());setMessage('模板已删除，历史分析保留。');})}>确认删除模板</button></footer></LearningDialog>}</>;
}
