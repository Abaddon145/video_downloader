import test from 'node:test';
import assert from 'node:assert/strict';
import {templateLabel,sectionsFromText,validateTemplateForm} from './templates.ts';
test('template labels retain frozen identity and treat legacy records explicitly',()=>{
 assert.equal(templateLabel({}), '通用总结 · 旧版');
 assert.equal(templateLabel({templateSnapshot:{name:'课程学习',version:2}}),'课程学习 · v2');
 assert.equal(templateLabel({templateId:'custom-old',templateVersion:3}),'历史模板 · v3');
});
test('custom template form bounds section names and Markdown placeholders',()=>{
 assert.deepEqual(sectionsFromText('学习目标\n\n练习题'),[{id:'section-1',title:'学习目标'},{id:'section-2',title:'练习题'}]);
 const form={name:'课程模板',description:'',focus:'学习重点',sections:sectionsFromText('学习目标'),markdownTemplate:'# {{title}}\n{{sections}}'};
 assert.equal(validateTemplateForm(form),null);
 assert.match(validateTemplateForm({...form,sections:sectionsFromText('目标\n目标')})??'',/重复/);
 assert.match(validateTemplateForm({...form,markdownTemplate:'{{include}}'})??'',/占位符/);
 assert.match(validateTemplateForm({...form,markdownTemplate:'{{title'})??'',/占位符/);
 assert.match(validateTemplateForm({...form,markdownTemplate:'<script>alert(1)</script>'})??'',/HTML/);
 assert.match(validateTemplateForm({...form,name:''})??'',/名称/);
});
