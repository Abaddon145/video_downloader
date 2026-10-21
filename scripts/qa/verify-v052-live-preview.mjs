import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installKnowledgeFixture} from './fixtures/knowledge.mjs';
import {installGeneralTemplateFixture} from './fixtures/general-template.mjs';

const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'docs/evidence/v0.5.2/phase3/ui';
mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[],errors=[];
const source='\uFEFF---\r\ntitle: "[[literal]]"\r\ntags: [wiki, yaml]\r\n---\r\n\r\n# Live preview\r\n\r\n普通 Markdown 行\r\n\r\n[[映流/目标#重点|关联知识]]\r\n\r\n```html\r\n<script>window.MARKDOWN_EXECUTED=true</script>\r\n```\r\n';
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
 await context.addInitScript(installKnowledgeFixture);await context.addInitScript(installGeneralTemplateFixture);
 await context.addInitScript(text=>{window.kb={configured:true,rev:1,notes:{'映流/课程总结.md':text,'映流/第三.md':'# 第三篇笔记','映流/目标.md':'# 目标\n## 重点\n知识归用户所有。'}};window.qaExpectedSource=text;window.qaWrites=[];const original=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{if(command==='knowledge_write')window.qaWrites.push(structuredClone(args));return original(command,args);};},source);
 const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));page.setDefaultTimeout(10000);
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5270');
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'知识库',exact:true}).click();
 await page.getByRole('button',{name:'展开文件夹 映流',exact:true}).click();await page.getByRole('button',{name:'打开笔记 课程总结.md',exact:true}).click();
 const editor=page.getByRole('textbox',{name:'Markdown正文',exact:true});await editor.waitFor();
 assert.equal(await page.getByRole('button',{name:'实时预览',exact:true}).getAttribute('aria-pressed'),'true');
 await page.locator('.cm-content [data-knowledge-target]').waitFor();assert.equal(await page.evaluate(()=>window.MARKDOWN_EXECUTED),undefined);assert.equal(await page.locator('.knowledge-live-shell script').count(),0);
 await page.getByRole('button',{name:'源码',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.cm-content')?.textContent.includes('title: "[[literal]]"'));
 assert.ok((await editor.textContent()).includes('[[映流/目标#重点|关联知识]]'));assert.ok((await editor.textContent()).includes('<script>window.MARKDOWN_EXECUTED=true</script>'));
 checks.push('default live mode renders safe Wiki preview while source mode preserves literal YAML, Wiki syntax, and code text');

 await editor.click();await page.keyboard.press('Control+Home');for(let i=0;i<7;i++)await page.keyboard.press('ArrowDown');await page.keyboard.press('Home');await page.keyboard.press('Shift+End');await page.getByRole('toolbar',{name:'Markdown格式工具'}).getByRole('button',{name:'加粗'}).click();
 await page.waitForFunction(()=>document.querySelector('.cm-content')?.textContent.includes('**普通 Markdown 行**'));
 const formatted=source.replace('普通 Markdown 行','**普通 Markdown 行**');await page.getByRole('button',{name:'保存',exact:true}).click();await page.waitForFunction(expected=>window.qaWrites.some(w=>w.text===expected),formatted);
 const saved=await page.evaluate(()=>window.qaWrites.at(-1));assert.equal(saved.text,formatted);assert.equal(saved.expectedHash,`mock:${source}`);
 await editor.click();await page.keyboard.press('Control+z');await page.waitForFunction(()=>document.querySelector('.cm-content')?.textContent.includes('普通 Markdown 行'));assert.ok((await editor.textContent()).includes('普通 Markdown 行'));assert.ok(!(await editor.textContent()).includes('**普通 Markdown 行**'));
 checks.push('formatting is a CodeMirror undoable transaction and save sends BOM/CRLF source with the expected hash');

 await page.getByRole('button',{name:'实时预览',exact:true}).click();await editor.click();await page.keyboard.press('Control+Home');for(let i=0;i<9;i++)await page.keyboard.press('ArrowDown');await page.keyboard.press('Home');await page.keyboard.press('Shift+End');
 const compact=page.getByRole('toolbar',{name:'选中文字操作'});await compact.waitFor();await compact.getByRole('button',{name:'加粗'}).click();
 assert.equal(await page.evaluate(()=>window.calls.some(c=>c.command==='knowledge_resolve')),false);assert.equal(await page.locator('.knowledge-note-header strong').getAttribute('title'),'映流/课程总结.md');
 checks.push('selection toolbar action keeps the current note and does not resolve or open the selected Wiki link');

 await editor.click();await page.keyboard.press('Control+End');await page.keyboard.insertText('\n本机草稿');await page.waitForFunction(()=>document.querySelector('.cm-content')?.textContent.includes('本机草稿'));
 const writesBefore=await page.evaluate(()=>window.qaWrites.length);
 await page.evaluate(()=>{window.kb.notes['映流/课程总结.md']='# 外部只读版本';window.kb.rev++;window.emitAI({id:'映流/课程总结.md',vaultId:'vault-test'},'knowledge-file-changed');});
 const conflict=page.getByRole('dialog',{name:'文件已在其他程序中修改',exact:true});await conflict.waitFor();assert.ok((await editor.textContent()).includes('本机草稿'));assert.equal(await page.evaluate(()=>window.kb.notes['映流/课程总结.md']),'# 外部只读版本');assert.equal(await page.evaluate(()=>window.qaWrites.length),writesBefore);
 await page.screenshot({animations:'disabled',path:`${out}/live-preview-conflict-1440.png`});checks.push('external read-only conflict keeps both versions and performs no write before an explicit resolution');

 await page.emulateMedia({reducedMotion:'reduce'});await page.screenshot({animations:'disabled',path:`${out}/live-preview-reduced-motion.png`});
 await page.setViewportSize({width:880,height:900});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({animations:'disabled',path:`${out}/live-preview-880.png`});
 checks.push('reduced-motion and 880px conflict views render without horizontal overflow');
 assert.deepEqual(errors,[]);writeFileSync(`${out}/checks.json`,JSON.stringify({mocked:true,passed:true,checks,pageErrors:errors},null,2));console.log(`PASS ${checks.length} live-preview UI groups`);
}catch(error){for(const context of browser.contexts())for(const page of context.pages()){console.error((await page.locator('body').innerText()).slice(-4000));await page.screenshot({animations:'disabled',path:`${out}/failure.png`});}throw error;}finally{await browser.close();}
