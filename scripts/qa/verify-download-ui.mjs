import { createRequire } from 'node:module';
import { mkdirSync, writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'artifacts/download-ui';
mkdirSync(out, { recursive:true });
const browser = await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const context = await browser.newContext({viewport:{width:1200,height:780}});
await context.route('**/*', route => new URL(route.request().url()).hostname === '127.0.0.1' ? route.continue() : route.abort());
await context.addInitScript(() => {
  window.isTauri = true;
  const callbacks = {}, listeners = {}; let next = 1;
  const states = ['queued','downloading','processing','paused','failed','completed','cancelled'];
  const task = (status,i) => ({id:String(i),status,request:{url:`https://example.com/video/${i}`,title:`任务 ${i}`,kind:'video',videoMode:'compatible',maxHeight:0,subtitleLanguages:[],thumbnail:null},progress:{percent:20,downloaded:100,total:500,speed:10,eta:40},phase:'测试任务',outputDir:'C:/downloads',files:[],error:null,logs:[],createdAt:i,finishedAt:null,queueOrder:0});
  window.testSnapshot = {settings:{downloadDir:'C:/downloads',concurrency:2,cookieMode:'none',browser:'edge',browserProfile:'',hasCookieFile:false,proxyEnabled:false,proxyUrl:'',autoCheckCoreUpdate:true,lastCoreUpdateCheck:null,cookieSummary:null,downloadPresets:[]},tasks:states.map(task),engine:{ready:true,version:'2026.01.01',ffmpegVersion:'FFmpeg',denoVersion:'Deno',error:null},notice:null,updating:false,previewing:false,coreUpdate:null};
  if (location.search.includes('imported-cookie-fixture')) {
    Object.assign(window.testSnapshot.settings,{cookieMode:'file',hasCookieFile:true,cookieSummary:{count:1,sessionCount:1,earliestExpiry:null,latestExpiry:null}});
    Object.assign(window.testSnapshot.tasks[0],{status:'failed',error:'Failed to decrypt with DPAPI',finishedAt:10});
  }
  window.testCalls = [];
  window.testEmit = (event,payload) => (listeners[event] || []).forEach(id => callbacks[id]?.({event,payload:structuredClone(payload)}));
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener:()=>{}};
  window.__TAURI_INTERNALS__ = {
    transformCallback(fn) { const id=next++;callbacks[id]=fn;return id; },
    async invoke(command,args={}) {
      window.testCalls.push(command);
      if (command === 'plugin:event|listen') { (listeners[args.event] ||= []).push(args.handler);return args.handler; }
      if (command === 'plugin:event|unlisten') { delete callbacks[args.eventId]; return; }
      if (command === 'get_snapshot') return structuredClone(window.testSnapshot);
      if (command === 'plugin:dialog|open') return 'C:/offline-cookie.txt';
      if (command === 'import_cookies') {
        if (window.testImportFailure) throw 'Cookie 文件格式无效';
        Object.assign(window.testSnapshot.settings,{cookieMode:'file',hasCookieFile:true,cookieSummary:{count:2,sessionCount:1,earliestExpiry:4102444800,latestExpiry:4102444800}});
        window.testEmit('snapshot-updated',window.testSnapshot);return;
      }
      if (command === 'batch_task_action') {
        return {succeeded:args.ids.filter(id => id==='0' || id==='1'),skipped:args.ids.filter(id=>id!=='0'&&id!=='1').map(id=>({id,reason:'合并或转换阶段不可暂停'})),failed:[]};
      }
      if (command === 'save_download_presets') {
        window.testSnapshot.settings.downloadPresets=structuredClone(args.presets);
        window.testEmit('snapshot-updated',window.testSnapshot);
        return structuredClone(window.testSnapshot.settings);
      }
      if (command === 'enqueue_downloads') {
        if (args.requests.some(request=>request.filenameTemplate.includes('{未知}'))) throw '未知文件名变量；可使用：标题、作者、日期、播放列表序号、视频ID';
        window.testQueuedRequests=structuredClone(args.requests);
        return args.requests.map((request,index)=>({...task('queued',100+index),request:structuredClone(request)}));
      }
      if (command === 'preview_sources') {
        if (!window.testPreviewMode) { await new Promise(resolve=>window.finishTestPreview=resolve);return []; }
        const results=args.urls.map(url=>({url,error:null,preview:{url,title:'离线预设验收列表',thumbnail:null,duration:120,uploader:'测试作者',site:'离线测试',isPlaylist:true,entries:[7,9].map(index=>({id:String(index),url:`https://example.com/playlist/${index}`,title:`示例视频 ${index}`,duration:60,thumbnail:null,playlistIndex:index})),formats:[],subtitles:[{language:'en',automatic:false}],videoCodec:'avc1',audioCodec:'mp4a',compatible:true,filesize:1024}}));
        results.forEach(result=>window.testEmit('preview-result',result));return structuredClone(results);
      }
      if (command === 'cancel_preview') { window.finishTestPreview?.();return; }
      return true;
    }
  };
});
// Existing download/media/native checks use an empty, typed AI state.
await context.addInitScript(()=>{
 const original=window.__TAURI_INTERNALS__.invoke;
 const settings={model:'deepseek-flash',defaultMode:'standard',whisperModel:'base',allowTranscript:true,allowImages:true,cleanTemporaryFiles:true,autoGenerateNotes:true,autoAnalyzeDownloads:false,chunkConcurrency:1};
 const empty={ai_get_settings:{settings,hasApiKey:false},ai_whisper_models:[],ai_model_download:null,ai_usage:{analyses:0,questions:0,models:[]},ai_cache_info:{bytes:0,clearableBytes:0},ai_item_statuses:{},ai_for_item:null,ai_item_analyses:{items:[],total:0,page:1},ai_course_transcript:{source:'none',analysisId:null,warning:null,nextStartMs:null,total:0,page:1,items:[]}};
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>Object.hasOwn(empty,command)?structuredClone(empty[command]):original(command,args);
});
const page=await context.newPage();
const checks=[];
async function check(name,fn) { await fn();checks.push(name); console.log(`PASS ${name}`); }
try {
  await page.goto((process.env.QA_BASE_URL??'http://127.0.0.1:5226'));
  const boxes=page.locator('.task-row>input[type=checkbox]');
  await boxes.first().waitFor();
  await check('Shift range, progress selection, toolbar counts',async()=>{
    await boxes.nth(0).check(); await boxes.nth(3).click({modifiers:['Shift']});
    assert.equal(await page.locator('.task-row>input:checked').count(),4);
    await page.evaluate(()=>window.testEmit('task-updated',{...window.testSnapshot.tasks[1],progress:{percent:45}}));
    assert.equal(await page.locator('.task-row>input:checked').count(),4);
    assert.equal(await page.getByRole('button',{name:'暂停（2）',exact:true}).isEnabled(),true);
  });
  await check('confirmation and expanded per-item results',async()=>{
    await page.getByRole('button',{name:'取消（4）',exact:true}).click();
    assert.match(await page.locator('dialog').innerText(),/4 项.*已下载的文件会保留/);
    await page.getByRole('button',{name:'返回',exact:true}).click();
    await page.getByRole('button',{name:'暂停（2）',exact:true}).click();
    await page.getByText('成功 2 项，跳过 2 项，失败 0 项',{exact:true}).waitFor();
    await page.getByText('查看逐项原因',{exact:true}).click();
    assert.match(await page.locator('dialog').innerText(),/合并或转换阶段不可暂停/);
    await page.getByRole('button',{name:'知道了',exact:true}).click();
  });
  await check('filtered select all and missing task removal',async()=>{
    await page.getByRole('button',{name:'清除选择',exact:true}).click();
    await page.getByLabel('筛选任务状态',{exact:true}).selectOption('paused');
    await page.getByLabel('全选当前筛选结果',{exact:true}).check();
    assert.equal(await page.locator('.task-row>input:checked').count(),1);
    await page.evaluate(()=>{window.testSnapshot.tasks=window.testSnapshot.tasks.filter(t=>t.id!=='3');window.testEmit('snapshot-updated',window.testSnapshot);});
    await page.getByText('已选 0 项，当前筛选共 0 项',{exact:true}).waitFor();
    await page.getByLabel('筛选任务状态',{exact:true}).selectOption('all');
  });
  await check('Ctrl+A in queue checkbox focus, Ctrl+L and Escape precedence',async()=>{
    await boxes.first().focus();await page.keyboard.press('Control+a');
    assert.equal(await page.locator('.task-row>input:checked').count(),6);
    await page.keyboard.press('Control+l');
    assert.equal(await page.getByRole('textbox',{name:'视频链接',exact:true}).evaluate(el=>document.activeElement===el),true);
    await page.getByRole('textbox',{name:'视频链接',exact:true}).fill('https://example.com/video');
    await page.keyboard.press('Control+Enter');await page.locator('dialog').waitFor();
    await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('dialog[open]'));
    assert.equal(await page.locator('.task-row>input:checked').count(),6);
    assert.ok(await page.evaluate(()=>window.testCalls.includes('cancel_preview')));
    await page.keyboard.press('Escape');await page.getByText('已选 0 项，当前筛选共 6 项',{exact:true}).waitFor();
  });
  await page.screenshot({path:`${out}/t1-queue.png`,fullPage:true});
  await check('update notice enters existing guarded workflow without automatic installation',async()=>{
    await page.evaluate(()=>{window.testSnapshot.coreUpdate={version:'2099.01.01',currentVersion:'2026.01.01',available:true,publishedAt:''};window.testEmit('snapshot-updated',window.testSnapshot);});
    await page.getByRole('button',{name:'内核有新版本 2099.01.01，去更新',exact:true}).click();
    await page.getByRole('checkbox',{name:'启动后检查内核更新',exact:true}).waitFor();
    assert.equal(await page.getByRole('checkbox',{name:'启动后检查内核更新',exact:true}).isChecked(),true);
    assert.equal(await page.getByRole('button',{name:'更新内核',exact:true}).isDisabled(),true);
    assert.equal(await page.evaluate(()=>window.testCalls.includes('update_engine')),false);
    await page.screenshot({path:`${out}/t2-update.png`,fullPage:true});
  });
  await check('Cookie failure guide and expired file statistics hide all values',async()=>{
    await page.evaluate(()=>{window.testSnapshot.tasks[0].status='failed';window.testSnapshot.tasks[0].error='ERROR: Failed to decrypt with DPAPI.';window.testEmit('snapshot-updated',window.testSnapshot);});
    await page.getByText('浏览器登录数据无法解密',{exact:true}).waitFor({timeout:3000});
    await page.getByText('如何导出 Cookie 文件',{exact:true}).click();
    assert.match(await page.locator('.cookie-guide').innerText(),/UTF-8.*2 MiB/s);
    assert.match(await page.locator('.cookie-guide').innerText(),/不要分享，只导入自己的账号/);
    await page.evaluate(()=>{Object.assign(window.testSnapshot.settings,{cookieMode:'file',hasCookieFile:true,cookieSummary:{count:2,sessionCount:0,earliestExpiry:1,latestExpiry:2}});window.testEmit('snapshot-updated',window.testSnapshot);});
    await page.getByText('Cookie 已过期，请重新导出',{exact:true}).waitFor();
    assert.doesNotMatch(await page.locator('.cookie-statistics').innerText(),/synthetic-tripwire/);
    await page.screenshot({path:`${out}/t3-cookie.png`,fullPage:true});
  });
  await check('successful Cookie import dismisses old warnings while new failures and failed imports remain visible',async()=>{
    await page.getByRole('button',{name:'重新导入',exact:true}).click();
    const warning=page.getByText('浏览器登录数据无法解密',{exact:true});
    await warning.waitFor({state:'hidden',timeout:3000});
    await page.evaluate(()=>{window.testEmit('snapshot-updated',window.testSnapshot);window.testEmit('task-updated',window.testSnapshot.tasks[0]);});
    assert.equal(await warning.count(),0);
    await page.evaluate(()=>{window.testSnapshot.tasks[0].finishedAt=20;window.testEmit('snapshot-updated',window.testSnapshot);});
    await warning.waitFor();
    await page.evaluate(()=>window.testImportFailure=true);
    await page.getByRole('button',{name:'重新导入',exact:true}).click();
    await page.getByText('Cookie 文件格式无效',{exact:true}).waitFor();
    assert.equal(await warning.count(),1);
    await page.evaluate(()=>{window.testImportFailure=false;window.testSnapshot.tasks=window.testSnapshot.tasks.filter(task=>task.id!=='0');window.testEmit('snapshot-updated',window.testSnapshot);});
    await warning.waitFor({state:'hidden',timeout:3000});
  });
  await check('create a preset in settings and apply all choices in preview',async()=>{
    await page.getByRole('button',{name:'新建预设',exact:true}).click();
    const editor=page.locator('.preset-editor');
    await editor.getByLabel('预设名称',{exact:true}).fill('音频归档');
    await editor.getByRole('combobox',{name:/^下载类型/}).selectOption('audio');
    await editor.getByLabel('字幕语言代码',{exact:true}).fill('en');
    await editor.getByLabel('文件名模板',{exact:true}).fill('{作者}_{播放列表序号}_{标题}');
    await editor.getByLabel('按作者分目录',{exact:true}).check();
    await editor.getByRole('button',{name:'保存预设',exact:true}).click();
    await page.getByRole('button',{name:'编辑预设：音频归档',exact:true}).waitFor();
    await page.evaluate(()=>window.testPreviewMode='playlist');
    await page.locator('nav[aria-label="主导航"]').getByRole('button',{name:/下载/}).click();
    await page.getByRole('textbox',{name:'视频链接',exact:true}).fill('https://example.com/playlist');
    await page.keyboard.press('Control+Enter');
    const modal=page.locator('dialog[open]');
    await modal.getByText('离线预设验收列表',{exact:true}).waitFor();
    assert.equal(await modal.getByRole('combobox',{name:/^下载预设/}).inputValue(),'');
    assert.equal(await modal.getByLabel('文件名模板',{exact:true}).inputValue(),'');
    assert.equal(await modal.getByLabel('按作者分目录',{exact:true}).isChecked(),false);
    assert.equal(await modal.getByLabel('字幕语言代码',{exact:true}).inputValue(),'zh-Hans,en');
    await modal.getByRole('combobox',{name:/^下载预设/}).selectOption('0');
    assert.equal(await modal.getByRole('button',{name:/音频.*转换为 MP3/}).getAttribute('aria-pressed'),'true');
    assert.equal(await modal.getByLabel('字幕语言代码',{exact:true}).inputValue(),'en');
    assert.equal(await modal.getByLabel('文件名模板',{exact:true}).inputValue(),'{作者}_{播放列表序号}_{标题}');
    assert.equal(await modal.getByLabel('按作者分目录',{exact:true}).isChecked(),true);
    await page.screenshot({path:`${out}/t4-preview.png`,fullPage:true,animations:'disabled'});
    await modal.getByLabel('文件名模板',{exact:true}).scrollIntoViewIfNeeded();
    await page.screenshot({path:`${out}/t4-naming.png`,fullPage:true,animations:'disabled'});
  });
  await check('save preview choices, show rejected template in dialog, freeze playlist indices',async()=>{
    const modal=page.locator('dialog[open]');
    await modal.getByLabel('保存当前选项为预设',{exact:true}).fill('列表音频');
    await modal.getByRole('button',{name:/^保存预设/}).click();
    await modal.getByText('预设已保存，可在设置中编辑。',{exact:true}).waitFor();
    await modal.getByLabel('文件名模板',{exact:true}).fill('{未知}');
    await modal.getByRole('button',{name:'加入下载 · 2',exact:true}).click();
    await modal.getByRole('alert').waitFor();
    assert.match(await modal.getByRole('alert').innerText(),/未知文件名变量/);
    await modal.getByLabel('文件名模板',{exact:true}).fill('{作者}_{播放列表序号}_{标题}');
    await modal.getByRole('button',{name:'加入下载 · 2',exact:true}).click();
    await page.waitForFunction(()=>!document.querySelector('dialog[open]'));
    const requests=await page.evaluate(()=>window.testQueuedRequests);
    assert.equal(requests.length,2);
    assert.deepEqual(requests.map(request=>request.playlistIndex),[7,9]);
    for (const request of requests) {assert.equal(request.kind,'audio');assert.deepEqual(request.subtitleLanguages,['en']);assert.equal(request.byAuthor,true);assert.equal(request.filenameTemplate,'{作者}_{播放列表序号}_{标题}');}
  });
  await check('editing presets preserves queued requests and 20 disables creation',async()=>{
    await page.locator('nav[aria-label="主导航"]').getByRole('button',{name:'设置',exact:true}).click();
    await page.getByRole('button',{name:'编辑预设：音频归档',exact:true}).click();
    const editor=page.locator('.preset-editor');
    await editor.getByLabel('预设名称',{exact:true}).fill('视频归档');
    await editor.getByRole('combobox',{name:/^下载类型/}).selectOption('video');
    await editor.getByLabel('文件名模板',{exact:true}).fill('{日期}_{视频ID}');
    await editor.getByRole('button',{name:'保存预设',exact:true}).click();
    await page.getByRole('button',{name:'编辑预设：视频归档',exact:true}).waitFor();
    const requests=await page.evaluate(()=>window.testQueuedRequests);
    assert.equal(requests[0].kind,'audio');assert.equal(requests[0].filenameTemplate,'{作者}_{播放列表序号}_{标题}');
    await page.getByRole('heading',{name:'下载预设',exact:true}).scrollIntoViewIfNeeded();
    await page.screenshot({path:`${out}/t4-settings.png`,fullPage:true,animations:'disabled'});
    await page.evaluate(()=>{const preset=window.testSnapshot.settings.downloadPresets[0];window.testSnapshot.settings.downloadPresets=Array.from({length:20},(_,index)=>({...preset,name:`预设 ${index}`}));window.testEmit('snapshot-updated',window.testSnapshot);});
    assert.equal(await page.getByRole('button',{name:'新建预设',exact:true}).isDisabled(),true);
  });
  await check('preset quality absent from source formats remains visibly selected and frozen',async()=>{
    await page.evaluate(()=>{window.testSnapshot.settings.downloadPresets[0].maxHeight=1440;window.testEmit('snapshot-updated',window.testSnapshot);});
    await page.locator('nav[aria-label="主导航"]').getByRole('button',{name:/下载/}).click();
    await page.getByRole('textbox',{name:'视频链接',exact:true}).fill('https://example.com/playlist');
    await page.keyboard.press('Control+Enter');
    const modal=page.locator('dialog[open]');
    await modal.getByText('离线预设验收列表',{exact:true}).waitFor();
    await modal.getByRole('combobox',{name:/^下载预设/}).selectOption('0');
    assert.equal(await modal.getByRole('combobox',{name:/^最高画质/}).inputValue(),'1440');
    assert.match(await modal.getByRole('combobox',{name:/^最高画质/}).locator('option:checked').innerText(),/1440p/);
    await modal.getByRole('button',{name:'加入下载 · 2',exact:true}).click();
    await page.waitForFunction(()=>!document.querySelector('dialog[open]'));
    assert.equal(await page.evaluate(()=>window.testQueuedRequests[0].maxHeight),1440);
  });
  await check('small-window layout and keyboard focus stay inside preview',async()=>{
    await page.setViewportSize({width:880,height:600});
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.locator('nav[aria-label="主导航"]').getByRole('button',{name:/下载/}).click();
    await page.getByRole('textbox',{name:'视频链接',exact:true}).fill('https://example.com/playlist');
    await page.keyboard.press('Control+Enter');
    const modal=page.locator('dialog[open]');
    await modal.getByText('离线预设验收列表',{exact:true}).waitFor();
    await modal.getByLabel('文件名模板',{exact:true}).focus();
    for (let index=0;index<15;index++) {await page.keyboard.press('Tab');assert.equal(await page.evaluate(()=>!!document.activeElement?.closest('dialog[open]')),true);}
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
    await page.screenshot({path:`${out}/t4-small.png`,fullPage:true,animations:'disabled'});
    await page.keyboard.press('Escape');
  });
  await check('restart with imported Cookie does not revive historical warnings and still shows new failures',async()=>{
    await page.goto((process.env.QA_BASE_URL??'http://127.0.0.1:5226')+'/?imported-cookie-fixture');
    await page.locator('nav[aria-label="主导航"]').getByRole('button',{name:'设置',exact:true}).click();
    await page.getByRole('button',{name:'重新导入',exact:true}).waitFor();
    const warning=page.getByText('浏览器登录数据无法解密',{exact:true});
    assert.equal(await warning.count(),0);
    await page.evaluate(()=>{window.testSnapshot.tasks[0].finishedAt=20;window.testEmit('task-updated',window.testSnapshot.tasks[0]);});
    await warning.waitFor();
  });
} finally { writeFileSync(`${out}/ui-checks.json`,JSON.stringify({kind:'离线浏览器交互测试，模拟 Tauri IPC，不代表 Windows 下载实机验收',checks},null,2));await browser.close(); }
