import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installLearningLayoutFixture} from './fixtures/learning-layout.mjs';
import {defaultWorkspace} from '../../src/features/workspace/defaults.ts';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'artifacts/v051/workspace-persistence';mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[];
async function setup(options={}){
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 await context.route('**/*',r=>new URL(r.request().url()).hostname==='127.0.0.1'?r.continue():r.abort());
 await context.addInitScript(installLearningLayoutFixture);
  await context.addInitScript(({slow,fail,initial,holdSave,failLoad})=>{
  if(initial&&!localStorage.getItem('qa-workspace'))localStorage.setItem('qa-workspace',JSON.stringify(initial));
  const invoke=window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke=async(command,args)=>{
   if(command==='workspace_get'&&failLoad&&!window.allowWorkspaceLoad)throw new Error('QA temporary read failure');
   if(command==='workspace_get'&&slow)return new Promise(resolve=>{window.releaseWorkspace=()=>resolve(JSON.parse(localStorage.getItem('qa-workspace')||'null'));});
   if(command==='workspace_save'&&fail){window.calls.push({command,args});throw new Error('QA disk full');}
   if(command==='workspace_save'&&holdSave)return new Promise(()=>{});
   return invoke(command,args);
  };
 },options);
 const page=await context.newPage();await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5260');
 return {context,page};
}
const openLibrary=async page=>{await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();await page.locator('.library-view-settings summary').click();};
try{
 {
  const context=await browser.newContext({viewport:{width:1440,height:1000}}),page=await context.newPage();
  await context.route('**/*',r=>new URL(r.request().url()).hostname==='127.0.0.1'?r.continue():r.abort());
  await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5260');
  await page.waitForFunction(()=>document.querySelector('.workspace-save-error')||document.querySelector('nav[aria-label="主导航"]'));
  assert.equal(await page.getByRole('navigation',{name:'主导航'}).count(),1,'browser-only UI preview must remain usable without a Rust bridge');
  checks.push('browser-only preview remains usable without a desktop settings file');await context.close();
 }
 {
  const initial=defaultWorkspace();initial.currentLayout.library.categoryWidth=340;initial.currentLayout.player.panelSize=520;
  const {context,page}=await setup({failLoad:true,initial});
  await page.getByRole('status').filter({hasText:'QA temporary read failure'}).waitFor();
  assert.equal(await page.getByRole('navigation',{name:'主导航'}).count(),0,'a rejected initial read must not expose editable defaults');
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='workspace_save').length),0,'a read failure must never save default settings');
  await page.evaluate(()=>window.allowWorkspaceLoad=true);
  await page.getByRole('button',{name:'重新读取工作区'}).click();await openLibrary(page);
  assert.equal(await page.getByLabel('分类栏宽度',{exact:true}).inputValue(),'340');
  assert.equal(await page.evaluate(()=>JSON.parse(localStorage.getItem('qa-workspace')).currentLayout.player.panelSize),520);
  checks.push('failed initial read gates editing and retries reading without overwriting saved settings');await context.close();
 }
 {
  const {context,page}=await setup({holdSave:true});await openLibrary(page);
  await page.getByLabel('学习库显示方式').selectOption('list');
  assert.equal(await page.getByLabel('学习库显示方式').inputValue(),'list','the view control must accept the edit');
  const pendingBefore=await page.evaluate(()=>JSON.parse(localStorage.getItem('yingliu-workspace-pending-v1')||'null')?.currentLayout.library.view);
  assert.equal(pendingBefore,'list','the pending layout must be durable before returning from the edit');
  await page.reload();await openLibrary(page);
  assert.equal(await page.getByLabel('学习库显示方式').inputValue(),'list','a reload before the debounce expires must retain the selected view');
  checks.push('immediate reload retains a pending layout before the Rust save finishes');await context.close();
 }
 {
  const initial=defaultWorkspace();initial.currentLayout.library.categoryWidth=340;initial.currentLayout.player.panelSize=520;
  const {context,page}=await setup({slow:true,initial});
  await page.waitForFunction(()=>typeof window.releaseWorkspace==='function');
  const canEdit=await page.getByRole('navigation',{name:'主导航'}).count();
  if(canEdit){await openLibrary(page);await page.getByLabel('学习库显示方式').selectOption('list');}
  await page.evaluate(()=>window.releaseWorkspace());
  if(!canEdit){await openLibrary(page);await page.getByLabel('学习库显示方式').selectOption('list');}
  assert.equal(await page.getByLabel('分类栏宽度',{exact:true}).inputValue(),'340','loading must not replace previously saved settings with defaults plus one edit');
  checks.push('delayed initial loading preserves unrelated saved category and player sizes');await context.close();
 }
 {
  const {context,page}=await setup({fail:true});await openLibrary(page);
  await page.getByLabel('学习库显示方式').selectOption('list');
  await page.getByRole('status').filter({hasText:'布局尚未写入磁盘'}).waitFor();
  assert.equal(await page.getByLabel('学习库显示方式').inputValue(),'list');
  await page.screenshot({path:`${out}/save-failure.png`});
  checks.push('disk failure remains visible and keeps the local pending layout');await context.close();
 }
 writeFileSync(`${out}/checks.json`,JSON.stringify({mocked:true,passed:true,checks},null,2));
}finally{await browser.close();}
