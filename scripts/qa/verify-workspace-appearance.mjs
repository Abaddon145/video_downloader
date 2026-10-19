import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installLearningLayoutFixture} from './fixtures/learning-layout.mjs';
import {installAiTemplateFixture} from './fixtures/ai-templates.mjs';

const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'artifacts/v051/workspace-appearance';mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[];
async function open({width=1440,height=1000,deviceScaleFactor=1}={}){
 const context=await browser.newContext({viewport:{width,height},deviceScaleFactor});
 await context.route('**/*',r=>new URL(r.request().url()).hostname==='127.0.0.1'?r.continue():r.abort());
 await context.addInitScript(installLearningLayoutFixture);
 await context.addInitScript(installAiTemplateFixture);
 await context.addInitScript(()=>{
  const original=window.__TAURI_INTERNALS__.invoke;
  window.qaImportResponse=null;window.qaExportError='';window.qaSaveError='';
  window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
   if(command==='workspace_import'){
    window.calls.push({command,args});const response=window.qaImportResponse;
    if(response?.error)throw new Error(response.error);
    return response?.settings??null;
   }
   if(command==='workspace_export'){
    window.calls.push({command,args});if(window.qaExportError)throw new Error(window.qaExportError);return true;
   }
   if(command==='workspace_save'&&window.qaSaveError){window.calls.push({command,args});throw new Error(window.qaSaveError);}
   return original(command,args);
  };
 });
 const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5260');
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'设置',exact:true}).click();
 try{await page.getByRole('heading',{name:'外观与工作区',exact:true}).waitFor({timeout:6000});}catch(e){console.log({errors,body:await page.locator('body').innerText()});throw e;}
 return {context,page,errors};
}
const saved=page=>page.evaluate(()=>JSON.parse(localStorage.getItem('qa-workspace')||'null'));
const applyInput=async(locator,value)=>locator.evaluate((el,v)=>{Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(el,String(v));el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));},value);
const flush=page=>page.waitForFunction(()=>{const p=localStorage.getItem('yingliu-workspace-pending-v1');return window.calls.some(c=>c.command==='workspace_save')&&!p;});
try{
 const {context,page,errors}=await open();const panel=page.locator('.workspace-settings');
 const theme=page.getByLabel('主题',{exact:true}),accent=page.getByLabel('强调色',{exact:true}),font=page.getByLabel('字体大小',{exact:true}),density=page.getByLabel('信息密度',{exact:true}),sidebar=page.getByLabel('侧边栏宽度',{exact:true}),motion=page.getByLabel('动画',{exact:true});
 await theme.selectOption('light');await page.waitForFunction(()=>document.documentElement.dataset.theme==='light');
 await applyInput(accent,'#c2410c');
 await applyInput(font,18);await density.selectOption('compact');await applyInput(sidebar,260);await motion.selectOption('reduced');await flush(page);
 const appearance=await page.evaluate(()=>({theme:document.documentElement.dataset.theme,density:document.documentElement.dataset.density,motion:document.documentElement.dataset.motion,accent:getComputedStyle(document.documentElement).getPropertyValue('--workspace-accent').trim(),fontScale:getComputedStyle(document.documentElement).getPropertyValue('--workspace-font-scale').trim(),sidebarWidth:getComputedStyle(document.documentElement).getPropertyValue('--workspace-sidebar-width').trim(),grid:getComputedStyle(document.querySelector('.app-shell')).gridTemplateColumns}));
 assert.equal(appearance.theme,'light');assert.equal(appearance.density,'compact');assert.equal(appearance.motion,'reduced');assert.ok(appearance.accent.toLowerCase().includes('c2410c')||appearance.accent.toLowerCase().includes('rgb(194, 65, 12)'));
 assert.notEqual(appearance.fontScale,'');assert.match(appearance.sidebarWidth,/260/);assert.match(appearance.grid,/260px/);
 assert.equal((await saved(page)).appearance.fontSize,18);checks.push('theme, accent, font size, density, sidebar width and motion reach root DOM/CSS and persist');
 await page.screenshot({animations:'disabled',path:`${out}/workspace-light-18px.png`});
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();await page.locator('.library-configured-grid').waitFor();
 assert.equal(await page.evaluate(()=>document.documentElement.dataset.theme),'light');await page.screenshot({animations:'disabled',path:`${out}/library-light-18px.png`});
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'下载',exact:true}).click();await page.locator('.link-card').waitFor();await page.screenshot({animations:'disabled',path:`${out}/download-light-18px.png`});
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'设置',exact:true}).click();

 const preset=page.getByLabel('布局预设',{exact:true}),presetName=page.getByLabel('预设名称',{exact:true});
 const options=await preset.locator('option').allTextContents();for(const label of ['默认','视频学习','专注播放','批量管理','知识整理'])assert.ok(options.some(option=>option.includes(label)),`missing builtin preset ${label}`);
 await preset.selectOption('builtin.default');assert.equal(await panel.getByRole('button',{name:'删除预设',exact:true}).isDisabled(),true,'built-in preset cannot be deleted');
 await presetName.fill('QA 自定义工作区');await panel.getByRole('button',{name:'新建预设',exact:true}).click();
 await page.getByLabel('布局预设',{exact:true}).locator('option').filter({hasText:'QA 自定义工作区'}).waitFor({state:'attached'});
 await flush(page);
 const workspace=await saved(page),custom=workspace.presets.find(p=>p.name==='QA 自定义工作区');assert.ok(custom);assert.deepEqual(Object.keys(custom).sort(),['id','layout','name']);
 await preset.selectOption(custom.id);await density.selectOption('comfortable');await flush(page);await panel.getByRole('button',{name:'保存当前布局',exact:true}).click();
 assert.equal((await saved(page)).presets.find(p=>p.id===custom.id).layout.library.view,(await saved(page)).currentLayout.library.view);
 await flush(page);await presetName.fill('QA 重命名工作区');await panel.getByRole('button',{name:'重命名预设',exact:true}).click();await flush(page);assert.ok((await saved(page)).presets.some(p=>p.name==='QA 重命名工作区'));
 page.on('dialog',dialog=>dialog.accept());await panel.getByRole('button',{name:'删除预设',exact:true}).click();await page.waitForFunction(id=>!JSON.parse(localStorage.getItem('qa-workspace')).presets.some(p=>p.id===id),custom.id);
 checks.push('five built-ins are selectable and protected; custom layout presets create, update, rename and delete');

 const beforeInvalid=await saved(page);const importButton=panel.getByRole('button',{name:'导入布局配置',exact:true});
 for(const [caseName,error] of [['credential','配置包含未知字段 apiKey'],['future','不支持的工作区配置版本'],['corrupt','工作区配置不是有效 JSON']]){
  await page.evaluate(error=>window.qaImportResponse={error},error);await importButton.click();await page.getByRole('alert').filter({hasText:error}).waitFor();
  assert.deepEqual(await saved(page),beforeInvalid,`${caseName} import must preserve the existing layout`);
 }
 const imported=structuredClone(beforeInvalid);imported.currentLayout.library.view='list';
 await page.evaluate(settings=>window.qaImportResponse={settings},imported);await importButton.click();await page.waitForFunction(()=>JSON.parse(localStorage.getItem('qa-workspace')).currentLayout.library.view==='list');
 checks.push('credential, future-version and corrupt imports show errors and retain settings; valid import applies');

 await panel.getByRole('button',{name:'导出布局配置',exact:true}).click();
 const exportCall=await page.evaluate(()=>window.calls.filter(c=>c.command==='workspace_export').at(-1));assert.ok(exportCall);assert.deepEqual(Object.keys(exportCall.args).sort(),['settings']);
 assert.deepEqual(Object.keys(exportCall.args.settings).sort(),['appearance','currentLayout','presets','version']);
 assert.ok(!JSON.stringify(exportCall.args.settings).match(/api.?key|credential|cookie|proxy.?url/i));checks.push('export sends only validated WorkspaceSettings layout fields');

 await panel.getByRole('button',{name:'恢复默认工作区',exact:true}).click();await page.waitForFunction(()=>JSON.parse(localStorage.getItem('qa-workspace')).appearance.theme==='dark');
 assert.equal((await saved(page)).appearance.fontSize,14);checks.push('restore defaults resets appearance and current layout');
 await page.evaluate(()=>window.qaExportError='QA export failure');await panel.getByRole('button',{name:'导出布局配置',exact:true}).click();await page.getByRole('alert').filter({hasText:'QA export failure'}).waitFor();checks.push('native export failure is visible in the settings UI');
 await page.evaluate(()=>window.qaSaveError='QA disk full');await density.selectOption('compact');await page.getByRole('status').filter({hasText:'QA disk full'}).waitFor();checks.push('workspace write failure remains visible');
 await page.screenshot({animations:'disabled',path:`${out}/workspace-settings.png`});assert.deepEqual(errors,[]);await context.close();

 for(const scale of [1.25,1.5]){
  const narrow=await open({width:880,height:900,deviceScaleFactor:scale});
  assert.ok(await narrow.page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`${scale} simulated deviceScaleFactor should fit at 880 CSS px`);
  await narrow.page.screenshot({animations:'disabled',path:`${out}/workspace-880-dsf-${scale}.png`});assert.deepEqual(narrow.errors,[]);
  checks.push(`880 CSS px fits at simulated deviceScaleFactor ${scale}; this is browser emulation, not physical DPI validation`);await narrow.context.close();
 }
 writeFileSync(`${out}/checks.json`,JSON.stringify({mocked:true,passed:true,checks},null,2));console.log(`PASS ${checks.length} workspace appearance and preset UI groups (DPI scale factors are simulated)`);
}finally{await browser.close();}
