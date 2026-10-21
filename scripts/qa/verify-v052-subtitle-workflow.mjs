import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installKnowledgeFixture} from './fixtures/knowledge.mjs';
import {installGeneralTemplateFixture} from './fixtures/general-template.mjs';

const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE??'C:/Users/14582/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const out=process.env.QA_OUTPUT_DIR??'docs/evidence/v0.5.2/phase6/ui';
mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[],errors=[];
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
 const installSubtitleFixture=()=>{
  window.courseReady=true;
  window.kb={configured:true,rev:1,notes:{'映流/课程总结.md':'\uFEFF# 旧笔记\r\n\r\n原文 \t\r\n'}};
  const original=window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
   if(command==='learning_thumbnail_tasks')return [];
   if(command==='get_media_snapshot')return {tasks:[],ready:true,error:null};
   if(command==='ai_get_settings'){const view=await original(command,args);return {...view,settings:{...view.settings,allowTranscript:!window.blockTranscriptUpload}};}
   if(command==='ai_course_transcript'){
    window.calls.push({command,args});
    const all=Array.from({length:123},(_,i)=>({id:`s${i}`,analysisId:'a',segmentIndex:i,startMs:i*10000,endMs:i*10000+9000,text:i===112?'第113段：唯一目标字幕':`第${i+1}段：Niagara 粒子`}));
    const found=args.query?all.filter(row=>row.text.includes(args.query)):all;
    const page=args.currentMs==null?Math.max(1,Math.min(Math.ceil(found.length/50)||1,args.page)):Math.floor(args.currentMs/500000)+1;
    return {items:found.slice((page-1)*50,page*50),page,total:found.length,source:'subtitle',analysisId:null,warning:null,nextStartMs:found[page*50]?.startMs??null};
   }
   if(command==='knowledge_trash_note'){
    window.calls.push({command,args});
    const text=window.kb.notes[args.path];
    if(text===undefined)throw Error('笔记不存在');
    const bytes=new TextEncoder().encode(text),hash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(b=>b.toString(16).padStart(2,'0')).join('');
    if(hash!==args.expectedHash)throw Error('文件已被外部修改');
    delete window.kb.notes[args.path];return {vaultId:args.vaultId,path:args.path};
   }
   if(command==='knowledge_write'&&window.forceWriteConflict){
    window.forceWriteConflict=false;window.calls.push({command,args});
    return {vaultId:args.vaultId,outcome:{status:'conflict',document:{path:args.path,text:window.kb.notes[args.path],hash:`mock:${window.kb.notes[args.path]}`,mtime:1}},indexWarning:null};
   }
   return original(command,args);
  };
 };
 await context.addInitScript({content:`(${installKnowledgeFixture.toString()})();(${installGeneralTemplateFixture.toString()})();(${installSubtitleFixture.toString()})();`});
 const page=await context.newPage();page.setDefaultTimeout(10000);page.on('pageerror',error=>errors.push(error.stack??error.message));
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5270');await page.getByRole('heading',{name:'下载',exact:true}).waitFor();
 await page.getByRole('button',{name:'学习库',exact:true}).click();await page.locator('.learning-card-open').click();await page.getByText('开始学习',{exact:true}).click();
 const sidebar=page.getByRole('complementary',{name:'课程学习面板'});await sidebar.waitFor();
 await sidebar.getByRole('tab',{name:'字幕',exact:true}).click();
 await sidebar.locator('.ai-transcript-line').first().waitFor();assert.equal(await sidebar.locator('.ai-transcript-line').count(),50);
 await sidebar.getByRole('button',{name:'下一页转录'}).click();await sidebar.getByText('第51段：Niagara 粒子').waitFor();
 await sidebar.getByRole('button',{name:'下一页转录'}).click();await sidebar.getByText('第113段：唯一目标字幕').waitFor();assert.equal(await sidebar.locator('.ai-transcript-line').count(),23);
 await sidebar.getByLabel('搜索整节课程字幕').fill('唯一目标');await page.waitForFunction(()=>document.querySelectorAll('.ai-transcript-line').length===1);await sidebar.getByText('第113段：唯一目标字幕').waitFor();assert.equal(await sidebar.locator('.ai-transcript-line').count(),1);
 assert.equal(await page.evaluate(()=>window.calls.findLast(c=>c.command==='ai_course_transcript').args.query),'唯一目标');
 checks.push('whole-course subtitle search filters before pages capped at 50');
 await sidebar.getByRole('checkbox',{name:'选择 1120000 毫秒字幕'}).check();
 await sidebar.getByRole('button',{name:'用所选字幕提问'}).click();
 await sidebar.getByRole('textbox',{name:'视频问题'}).waitFor();assert.match(await sidebar.getByRole('textbox',{name:'视频问题'}).inputValue(),/第113段：唯一目标字幕/);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='ai_ask_video').length),0);
 await page.evaluate(()=>{window.blockTranscriptUpload=true;});await sidebar.getByRole('button',{name:'发送视频问题'}).click();await sidebar.getByText(/当前已禁止发送字幕文本/).waitFor();assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='ai_ask_video').length),0);
 checks.push('disabled transcript upload blocks selected-text Send before the paid AI call');
 await page.evaluate(()=>{window.blockTranscriptUpload=false;});
 await sidebar.getByRole('button',{name:'发送视频问题'}).click();await page.waitForFunction(()=>window.calls.some(c=>c.command==='ai_ask_video'));
 checks.push('selected subtitle prefills a question without paid request until explicit Send');
 await sidebar.getByRole('tab',{name:'字幕',exact:true}).click();await sidebar.getByLabel('搜索整节课程字幕').fill('唯一目标');await page.waitForFunction(()=>document.querySelectorAll('.ai-transcript-line').length===1);await sidebar.getByRole('checkbox',{name:'选择 1120000 毫秒字幕'}).check();await sidebar.getByRole('button',{name:'保存到知识库'}).click();
 const dialog=page.getByRole('dialog',{name:'保存到知识库'});await dialog.waitFor();
 const preview=dialog.locator('.note-source-preview');assert.match(await preview.innerText(),/yingliu:\/\/video\/lesson\?t=1120000/);assert.match(await preview.innerText(),/startMs:1120000 endMs:1129000/);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='knowledge_create').length),0);
 await page.screenshot({animations:'disabled',path:`${out}/subtitle-preview-1440.png`});
 await dialog.getByRole('button',{name:'确认写入知识库'}).click();await dialog.waitFor({state:'hidden'});
 const created=await page.evaluate(()=>window.calls.findLast(c=>c.command==='knowledge_create'));
 assert.ok(created);assert.match(await page.evaluate(path=>window.kb.notes[path],created.args.path),/startMs:1120000 endMs:1129000/);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='knowledge_read').length),0);
 checks.push('new note requires preview and uses generated content hash for Undo without a post-create read');
 await page.evaluate(path=>{window.kb.notes[path]+='外部修改';},created.args.path);
 await page.getByRole('button',{name:'撤销',exact:true}).click();await page.getByText('未能撤销：创建课程摘录').waitFor();
 assert.ok(await page.evaluate(path=>window.kb.notes[path]?.includes('外部修改'),created.args.path));
 checks.push('new-note Undo rejects external edits');
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库'}).click();await page.locator('.learning-card-open').click();await page.getByText('开始学习',{exact:true}).click();await sidebar.waitFor();await sidebar.getByRole('tab',{name:'字幕'}).click();await sidebar.getByLabel('搜索整节课程字幕').fill('唯一目标');await page.waitForFunction(()=>document.querySelectorAll('.ai-transcript-line').length===1);await sidebar.getByRole('checkbox',{name:'选择 1120000 毫秒字幕'}).check();
 await sidebar.getByRole('button',{name:'保存到知识库'}).click();await dialog.getByRole('radio',{name:'追加到已有笔记'}).check();
 await dialog.getByLabel('已有笔记相对路径').fill('映流/课程总结.md');await dialog.getByRole('button',{name:'读取目标笔记'}).click();await dialog.getByText(/已读取原文/).waitFor();
 const originalNote='\uFEFF# 旧笔记\r\n\r\n原文 \t\r\n';
 assert.ok((await preview.textContent()).startsWith(originalNote));
 await page.evaluate(()=>{window.kb.notes['映流/课程总结.md']='外部版本';});
 await dialog.getByRole('button',{name:'确认写入知识库'}).click();await dialog.getByText(/目标笔记已被修改/).waitFor();
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='knowledge_write').length),0);
 await page.evaluate(text=>{window.kb.notes['映流/课程总结.md']=text;},originalNote);
 await dialog.getByRole('button',{name:'读取目标笔记'}).click();await dialog.getByRole('button',{name:'确认写入知识库'}).click();await dialog.waitFor({state:'hidden'});
 const write=await page.evaluate(()=>window.calls.findLast(c=>c.command==='knowledge_write'));
 assert.equal(write.args.expectedHash,`mock:${originalNote}`);assert.ok(write.args.text.startsWith(originalNote));
 checks.push('append preserves BOM, CRLF and trailing whitespace, and stale review cannot write');
 await page.getByRole('button',{name:'撤销',exact:true}).click();await page.waitForFunction(text=>window.kb.notes['映流/课程总结.md']===text,originalNote);
 checks.push('append Undo restores exact original bytes under the saved hash');
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库'}).click();await page.locator('.learning-card-open').click();await page.getByText('开始学习',{exact:true}).click();await sidebar.waitFor();
 const paid=await page.evaluate(()=>window.calls.filter(c=>['ai_ask_video','ai_start_analysis','ai_retry_analysis'].includes(c.command)).length);
 await sidebar.getByRole('button',{name:'将总结保存或追加到笔记'}).click();await dialog.waitFor();assert.match(await preview.textContent(),/AI 总结/);await dialog.getByRole('radio',{name:'追加到已有笔记'}).check();await dialog.getByLabel('已有笔记相对路径').fill('映流/课程总结.md');await dialog.getByRole('button',{name:'读取目标笔记'}).click();assert.match(await preview.textContent(),/AI 总结/);await page.evaluate(()=>{window.forceWriteConflict=true;});await dialog.getByRole('button',{name:'确认写入知识库'}).click();await dialog.getByText(/目标笔记已被修改/).waitFor();assert.equal(await page.evaluate(()=>window.kb.notes['映流/课程总结.md']),originalNote);await dialog.getByRole('button',{name:'关闭并丢弃草稿'}).click();assert.equal(await page.evaluate(()=>window.calls.filter(c=>['ai_ask_video','ai_start_analysis','ai_retry_analysis'].includes(c.command)).length),paid);
 await page.getByRole('button',{name:/后台任务/}).first().click();const tasks=page.getByRole('dialog',{name:'后台任务'});assert.match(await tasks.locator('.task-center-item').first().innerText(),/追加课程摘录.*失败/s);await tasks.getByRole('button',{name:'关闭任务中心'}).click();
 checks.push('summary append conflict stays unsaved and is marked failed in Task Center without another AI request');
 await page.screenshot({animations:'disabled',path:`${out}/subtitle-saved-1440.png`});
 assert.deepEqual(errors,[]);
 writeFileSync(`${out}/checks.json`,JSON.stringify({mocked:true,passed:true,checks,pageErrors:errors},null,2));
 console.log(`PASS ${checks.length} mocked subtitle workflow groups`);
}catch(error){console.error('PAGE ERRORS',errors);for(const context of browser.contexts())for(const page of context.pages()){console.error('SETTINGS',await page.evaluate(()=>window.qaAiSettings));console.error('CALLS',await page.evaluate(()=>window.calls?.slice(-20)));console.error((await page.locator('body').innerText()).slice(-4000));await page.screenshot({animations:'disabled',path:`${out}/failure.png`});}throw error;}finally{await browser.close();}
