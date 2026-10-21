import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installKnowledgeFixture} from './fixtures/knowledge.mjs';
import {installGeneralTemplateFixture} from './fixtures/general-template.mjs';
const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'docs/evidence/v0.5.2/phase2/ui';mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[],errors=[];
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
 await context.addInitScript(installKnowledgeFixture);await context.addInitScript(installGeneralTemplateFixture);
 await context.addInitScript(()=>{const original=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==='learning_search_items')return {items:[{id:'lesson',seriesId:'series',title:'GPU 碰撞课程',available:true,progress:{positionMs:751000}}],total:1,page:1};
  return original(command,args);
 };});
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(10000);
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5270');await page.getByRole('heading',{name:'下载',exact:true}).waitFor();
 await page.keyboard.press('Control+k');const palette=page.getByRole('dialog',{name:'命令面板',exact:true});await palette.waitFor();
 await palette.getByRole('combobox',{name:'搜索命令'}).fill('note');await palette.getByText('新建笔记',{exact:true}).waitFor();assert.match(await palette.innerText(),/请先/);
 await palette.getByRole('combobox',{name:'搜索命令'}).fill('打开历史');await page.keyboard.press('Enter');await page.getByRole('heading',{name:'下载历史',exact:true}).waitFor();await palette.waitFor({state:'hidden'});
 checks.push('CtrlK bilingual search, disabled reasons, Enter runs actual navigation');
 await page.keyboard.press('Control+k');const first=await palette.locator('[role=option]').first().innerText();assert.match(first,/打开历史/);const activeBefore=await palette.getByRole('combobox',{name:'搜索命令'}).getAttribute('aria-activedescendant');await page.keyboard.press('ArrowDown');assert.notEqual(await palette.getByRole('combobox',{name:'搜索命令'}).getAttribute('aria-activedescendant'),activeBefore);await page.keyboard.press('ArrowUp');assert.equal(await palette.getByRole('combobox',{name:'搜索命令'}).getAttribute('aria-activedescendant'),activeBefore);await page.keyboard.press('Tab');assert.notEqual(await page.evaluate(()=>document.activeElement?.tagName),'BODY');await page.keyboard.press('Escape');checks.push('recent order, arrow selection, Tab focus and Esc');
 await page.getByRole('button',{name:'快捷键',exact:true}).click();const help=page.getByRole('dialog',{name:'快捷键帮助与设置'});await help.getByRole('textbox',{name:'搜索快捷键'}).fill('打开命令面板');await help.getByRole('button',{name:'修改',exact:true}).click();await help.getByRole('textbox',{name:'打开命令面板 的快捷键'}).fill('Ctrl+Shift+1');await help.getByRole('button',{name:'保存',exact:true}).click();await page.keyboard.press('Escape');await page.keyboard.press('Control+Shift+1');await palette.waitFor();await page.keyboard.press('Escape');checks.push('custom shifted-digit binding opens the palette');
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'知识库',exact:true}).click();await page.getByRole('button',{name:'选择已有知识库',exact:true}).click();await page.getByRole('button',{name:'展开文件夹 映流',exact:true}).click();await page.getByRole('button',{name:'打开笔记 课程总结.md',exact:true}).click();const editor=page.getByRole('textbox',{name:'Markdown正文',exact:true});await editor.click();await page.keyboard.press('Control+a');await page.keyboard.insertText('# 快捷键保存\n\n普通 Markdown。');await page.keyboard.press('Control+s');await page.waitForFunction(()=>window.kb.notes['映流/课程总结.md']==='# 快捷键保存\n\n普通 Markdown。');checks.push('scoped CtrlS calls the existing guarded save without stealing editor text');
 await page.keyboard.press('Control+p');const quick=page.getByRole('dialog',{name:'搜索课程与笔记'});await quick.getByRole('textbox',{name:'搜索课程与知识笔记'}).fill('GPU');await quick.getByText('GPU 碰撞课程',{exact:true}).waitFor();await page.screenshot({path:out+'/quick-search.png'});await page.keyboard.press('Escape');checks.push('CtrlP queries actual bounded course/knowledge APIs');
 await page.keyboard.press('Control+Shift+1');await palette.getByRole('combobox',{name:'搜索命令'}).fill('保存当前笔记');await page.screenshot({path:out+'/command-palette.png'});await page.keyboard.press('Escape');
 assert.deepEqual(errors,[]);writeFileSync(out+'/checks.json',JSON.stringify({mockedTauri:true,checks,pageErrors:errors},null,2));console.log(`PASS ${checks.length} mocked browser groups`);
}finally{await browser.close();}
