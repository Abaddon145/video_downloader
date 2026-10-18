import type {AnalysisTemplate} from '../../types/ai.ts';
export function templateLabel(analysis:{templateId?:string;templateVersion?:number;templateSnapshot?:{name:string;version:number}|null}){
 if(analysis.templateSnapshot)return `${analysis.templateSnapshot.name} · v${analysis.templateSnapshot.version}`;
 return analysis.templateId&&analysis.templateId!=='general'?`历史模板 · v${analysis.templateVersion??1}`:'通用总结 · 旧版';
}
export function sectionsFromText(text:string){return text.split('\n').map(title=>title.trim()).filter(Boolean).map((title,index)=>({id:`section-${index+1}`,title}));}
const placeholders=new Set(['templateName','title','oneSentenceSummary','overview','chapters','keyPoints','sections','tutorialSteps','terms','actionItems','links']);
export function validateTemplateForm(form:Pick<AnalysisTemplate,'name'|'description'|'focus'|'sections'|'markdownTemplate'>):string|null{
 if(!form.name.trim()||form.name.length>80)return '模板名称不能为空，最多80字';
 if(form.description.length>1000||!form.focus.trim()||form.focus.length>4000)return '请填写分析重点（最多4000字），说明最多1000字';
 if(!form.sections.length||form.sections.length>12||form.sections.some(s=>!s.title.trim()||s.title.length>80))return '请填写1～12个输出栏目，每行一个，最多80字';
 if(new Set(form.sections.map(s=>s.title.trim())).size!==form.sections.length)return '输出栏目不能重复';
 if(form.markdownTemplate.length>16000)return '知识笔记格式过长，最多16000字';
 if([...form.markdownTemplate.matchAll(/\{\{([^{}]*)\}\}/g)].some(match=>!placeholders.has(match[1])))return '知识笔记格式包含未知占位符';
 if(form.markdownTemplate.replace(/\{\{[^{}]*\}\}/g,'').includes('{{')||form.markdownTemplate.replace(/\{\{[^{}]*\}\}/g,'').includes('}}'))return '知识笔记格式包含未闭合占位符';
 if(/<\/?[a-zA-Z][^>]*>/.test(form.markdownTemplate))return '知识笔记格式不支持原始 HTML，请使用 Markdown';
 return null;
}
