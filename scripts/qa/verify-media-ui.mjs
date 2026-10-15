import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'artifacts/media-ui';mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const context=await browser.newContext({viewport:{width:1200,height:780},reducedMotion:'reduce'});
await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
await context.addInitScript(()=>{
 window.isTauri=true;const callbacks={},listeners={};let next=1;
 window.testCalls=[];window.testRequests=[];window.testMedia={tasks:[],ready:true,error:null};
 window.testSnapshot={settings:{downloadDir:'C:/downloads',concurrency:2,cookieMode:'none',browser:'edge',browserProfile:'',hasCookieFile:false,proxyEnabled:false,proxyUrl:'',autoCheckCoreUpdate:true,lastCoreUpdateCheck:null,cookieSummary:null,downloadPresets:[]},tasks:[],engine:{ready:true,version:'2026.10.09',ffmpegVersion:'FFmpeg test',denoVersion:'Deno',error:null},notice:null,updating:false,previewing:false,coreUpdate:null};
 window.testEmit=(event,payload)=>(listeners[event]||[]).forEach(id=>callbacks[id]?.({event,payload:structuredClone(payload)}));
 window.testInfo=path=>({path,fileName:path.split(/[\\/]/).pop(),size:1024*1024*128,duration:600,container:'matroska,webm',bitrate:1789569,videos:[{index:0,codec:'h264',profile:'High',width:1920,height:1080,fps:29.97,pixelFormat:'yuv420p',bitDepth:8,bitrate:1600000,attachedPicture:false,metadata:{title:'测试视频'}}],audios:[{index:1,codec:'aac',profile:'LC',sampleRate:48000,channels:2,channelLayout:'stereo',bitrate:192000,metadata:{language:'zho'}}],subtitles:[],otherStreams:0,metadata:{title:'离线媒体工具验收'}});
 window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
 window.__TAURI_INTERNALS__={convertFileSrc:()=> 'data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" width="160" height="90"/%3E',metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(fn){const id=next++;callbacks[id]=fn;return id;},async invoke(command,args={}){
  window.testCalls.push({command,args:structuredClone(args)});
  if(command==='plugin:event|listen'){(listeners[args.event]||=[]).push(args.handler);return args.handler;}
  if(command==='plugin:event|unlisten'){delete callbacks[args.eventId];return;}
  if(command==='get_media_settings')return {hardwareAcceleration:'auto',afterDownload:'none',enabledAt:0,presets:[{id:'phone',name:'手机视频',operation:'compress',outputFormat:'mp4',videoCodec:'h264',audioCodec:'aac',resolution:1080,quality:'balanced',hardwareAcceleration:'auto',generateThumbnails:false}]};
  if(command==='save_media_settings')return args.settings;
  if(command==='preview_media')return args.path;
  if(command==='media_thumbnails')return [];
  if(command==='discover_media')return args.paths;
  if(command==='get_snapshot')return structuredClone(window.testSnapshot);
  if(command==='get_media_snapshot'){const state=structuredClone(window.testMedia);if(location.search.includes('late-snapshot'))await new Promise(resolve=>(window.finishSnapshots||=[]).push(resolve));return state;}
  if(command==='plugin:dialog|open')return args.options?.directory?'C:/output':'C:/视频 测试/fixture.mkv';
  if(command==='probe_media'){
   if(args.path.includes('slow'))await new Promise(resolve=>window.finishSlowProbe=resolve);
   const info=window.testInfo(args.path);if(args.path.endsWith('.mp3'))info.videos=[];return info;
  }
  if(command==='create_media_task'){
   if(window.testCreateFailure)throw '保存目录不可写，请更换位置';
   window.testRequests.push(structuredClone(args.request));
   const task={id:String(window.testMedia.tasks.length+1),type:args.request.operation,inputPath:args.request.inputPath,outputPath:null,request:args.request,status:'queued',phase:'等待处理',progress:null,speed:null,processedTime:null,totalDuration:null,eta:null,error:null,logs:[],createdAt:Date.now(),finishedAt:null};
   window.testMedia.tasks.push(task);window.testEmit('media-snapshot-updated',window.testMedia);return structuredClone(task);
  }
  if(command==='cancel_media_task'||command==='retry_media_task'){const t=window.testMedia.tasks.find(t=>t.id===args.id);t.status=command==='cancel_media_task'?'cancelled':'queued';t.phase=t.status;window.testEmit('media-snapshot-updated',window.testMedia);return;}
  return true;
 }};
});
const page=await context.newPage(),checks=[],errors=[];page.on('pageerror',e=>errors.push(String(e)));
const panel=()=>page.getByRole('tabpanel');
async function check(name,fn){await fn();checks.push(name);console.log(`PASS ${name}`);}
const tool=async name=>page.getByRole('tab',{name,exact:true}).click();
const submit=async()=>{const n=await page.evaluate(()=>window.testRequests.length);await panel().getByRole('button',{name:/^(开始处理|截取当前帧)$/}).click();await page.waitForFunction(n=>window.testRequests.length===n+1,n);return page.evaluate(()=>window.testRequests.at(-1));};
const drop=async paths=>page.evaluate(paths=>window.testEmit('tauri://drag-drop',{paths,position:{x:20,y:20}}),paths);
try{
 await page.goto((process.env.QA_BASE_URL??'http://127.0.0.1:5226'));await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'媒体工具',exact:true}).click();
 await check('native picker, file summary and complete metadata',async()=>{
  await page.getByRole('button',{name:'选择媒体文件',exact:true}).click();await page.getByRole('heading',{name:'fixture.mkv',exact:true}).waitFor();
  assert.match(await page.locator('.media-info-card').innerText(),/1920 × 1080.*H264/s);
  await page.getByText('完整媒体信息 · 1 视频 / 1 音频 / 0 字幕',{exact:true}).click();assert.match(await page.locator('.media-track-content').innerText(),/yuv420p.*8-bit/s);
  await page.getByText('完整媒体信息 · 1 视频 / 1 音频 / 0 字幕',{exact:true}).click();await page.screenshot({path:`${out}/ui-summary.png`});
 });
 await check('transcode advanced fields and structured request',async()=>{
  await page.getByRole('checkbox',{name:'展开高级选项',exact:true}).check();await page.getByRole('combobox',{name:/^分辨率/}).selectOption('720');await page.getByRole('combobox',{name:/^帧率/}).selectOption('30');await page.getByLabel('质量 CRF',{exact:true}).fill('20');
  const r=await submit();assert.equal(r.operation,'transcode');assert.equal(r.height,720);assert.equal(r.fps,30);assert.equal(r.crf,20);assert.equal(r.videoCodec,'h264');assert.ok(!('args' in r));
  await panel().screenshot({path:`${out}/ui-transcode.png`});
 });
 await check('lossless remux and WebM copy-only option',async()=>{await tool('极速无损转换');await page.getByRole('combobox',{name:/^目标格式/}).selectOption('mkv');assert.equal((await submit()).operation,'remux');await tool('格式转换');await page.waitForFunction(()=>document.querySelector('option[value="webm"]').disabled);assert.equal(await page.locator('option[value="webm"]').getAttribute('disabled'),'');});
 await check('millisecond trim boundaries, duration and fast mode',async()=>{
  await tool('视频裁剪');await page.getByLabel('开始时间',{exact:true}).fill('00:01:00.000');await page.getByLabel('结束时间',{exact:true}).fill('00:02:00.000');assert.match(await page.locator('.media-fragment').innerText(),/00:01:00.000/);
  await page.getByRole('radio',{name:/极速无损裁剪/}).check();const r=await submit();assert.equal(r.start,60);assert.equal(r.end,120);assert.equal(r.trimMode,'fast');
  await page.getByLabel('结束时间',{exact:true}).fill('00:00:30.000');await panel().getByRole('button',{name:'开始处理',exact:true}).click();await page.getByText('裁剪时间必须满足：开始 < 结束 ≤ 媒体时长',{exact:true}).waitFor();
 });
 await check('audio extraction defaults to MP3 320 kbps',async()=>{await tool('提取音频');assert.equal(await page.getByRole('combobox',{name:/^音频码率/}).inputValue(),'320');const r=await submit();assert.equal(r.outputFormat,'mp3');assert.equal(r.audioBitrate,320);await panel().screenshot({path:`${out}/ui-audio.png`});});
 await check('screenshot timestamp and three image formats',async()=>{await tool('视频截图');await page.getByLabel('截图时间',{exact:true}).fill('00:00:10.500');const r=await submit();assert.equal(r.start,10.5);assert.equal(r.outputFormat,'png');assert.deepEqual(await page.getByRole('combobox',{name:/^目标格式/}).locator('option').evaluateAll(els=>els.map(e=>e.value)),['png','jpg','webp']);});
 await check('task progress, cancellation, restart, completed actions and log disclosure',async()=>{
  await page.evaluate(()=>{const t=window.testMedia.tasks[0];Object.assign(t,{status:'processing',phase:'正在转换',progress:35.5,speed:2.2,processedTime:213,totalDuration:600,eta:176,logs:['合成测试详情']});window.testEmit('media-snapshot-updated',window.testMedia);});
  const row=page.locator('.media-task-row').last();assert.match(await row.innerText(),/35.5%.*2.2x/s);await row.getByRole('button',{name:'取消',exact:true}).click();await row.getByRole('button',{name:'重新开始',exact:true}).click();
  await page.evaluate(()=>{Object.assign(window.testMedia.tasks[0],{status:'completed',phase:'已完成',progress:100,outputPath:'C:/output/result.mp4'});window.testEmit('media-snapshot-updated',window.testMedia);});
  await row.getByRole('button',{name:'打开文件',exact:true}).click();await row.getByRole('button',{name:'打开目录',exact:true}).click();await row.getByText('技术详情（已脱敏）',{exact:true}).click();assert.match(await row.locator('pre').innerText(),/合成测试详情/);
  const calls=await page.evaluate(()=>window.testCalls.filter(c=>c.command==='open_media_output'));assert.deepEqual(calls.map(c=>c.args.folder),[false,true]);
 });
 await check('output errors stay visible and retryable',async()=>{await page.evaluate(()=>window.testCreateFailure=true);await panel().getByRole('button',{name:'截取当前帧',exact:true}).click();await page.getByText('保存目录不可写，请更换位置',{exact:true}).waitFor();await page.evaluate(()=>window.testCreateFailure=false);});
 await check('native multiple-file drag opens batch and stale probe cannot replace latest file',async()=>{
  await drop(['C:/a.mkv','C:/b.mkv']);await page.getByText('2 个文件',{exact:true}).waitFor();
  await drop(['C:/slow.mkv']);await page.waitForFunction(()=>!!window.finishSlowProbe);await drop(['C:/latest.mkv']);await page.getByRole('heading',{name:'latest.mkv',exact:true}).waitFor();await page.evaluate(()=>window.finishSlowProbe());await page.waitForFunction(()=>!document.querySelector('[role="status"]'));assert.equal(await page.getByRole('heading',{name:'slow.mkv',exact:true}).count(),0);
 });
 await check('audio-only file disables video actions',async()=>{await drop(['C:/only.mp3']);await page.getByRole('heading',{name:'only.mp3',exact:true}).waitFor();assert.equal(await page.getByRole('tab',{name:'视频截图',exact:true}).isDisabled(),true);assert.equal(await page.getByRole('tab',{name:'提取音频',exact:true}).getAttribute('aria-selected'),'true');});
 await check('small window, keyboard focus and reduced motion',async()=>{
  await drop(['C:/fixture.mkv']);await page.getByRole('heading',{name:'fixture.mkv',exact:true}).waitFor();await page.setViewportSize({width:880,height:600});await page.getByRole('checkbox',{name:'展开高级选项',exact:true}).check();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.getByRole('combobox',{name:/^视频编码/}).focus();await page.keyboard.press('Tab');assert.equal(await page.getByRole('combobox',{name:/^分辨率/}).evaluate(e=>e===document.activeElement),true);
  assert.equal(await page.evaluate(()=>matchMedia('(prefers-reduced-motion: reduce)').matches),true);await page.screenshot({path:`${out}/ui-small.png`});
 });
 await check('disk-root sources retain their root output directory',async()=>{
  for(const path of ['C:/clip.mp4',String.fromCharCode(92,92,63,92)+'C:'+String.fromCharCode(92)+'clip.mp4']){
   await drop([path]);await page.getByRole('heading',{name:'clip.mp4',exact:true}).waitFor();const expected=path.includes('/')?'C:/':'C:'+String.fromCharCode(92);assert.equal(await page.getByLabel('媒体保存目录',{exact:true}).inputValue(),expected);
  }
 });
 await check('late initial snapshot cannot erase newer live task events',async()=>{
  const fresh=await context.newPage();await fresh.goto('http://127.0.0.1:5226?late-snapshot');await fresh.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'媒体工具',exact:true}).click();await fresh.waitForFunction(()=>window.finishSnapshots?.length);
  await fresh.evaluate(()=>window.__TAURI_INTERNALS__.invoke('create_media_task',{request:{inputPath:'C:/newest.mkv',operation:'remux',outputFormat:'mp4'}}));await fresh.locator('.media-task-row').waitFor();
  await fresh.evaluate(()=>{window.finishSnapshots.forEach(resolve=>resolve());});await fresh.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));assert.equal(await fresh.locator('.media-task-row').count(),1);await fresh.close();
 });
 await check('media preset applies 1080p choices, compressor 1440p and extra tool fields',async()=>{await page.getByRole('combobox',{name:'媒体预设',exact:true}).selectOption('phone');assert.equal(await page.getByRole('tab',{name:'视频压缩',exact:true}).getAttribute('aria-selected'),'true');assert.equal(await page.getByRole('combobox',{name:/^分辨率/}).inputValue(),'1080');await page.getByRole('combobox',{name:/^分辨率/}).selectOption('1440');assert.equal((await submit()).height,1440);await tool('视频抽帧');await page.getByRole('combobox',{name:'抽帧模式',exact:true}).selectOption('count');await page.getByLabel('图片数量',{exact:true}).fill('20');const f=await submit();assert.equal(f.frameCount,20);assert.equal(f.frameMode,'count');await tool('联系表');assert.equal((await submit()).sheetColumns,3);await tool('元数据');await page.getByLabel('标题',{exact:true}).fill('新的标题');assert.equal((await submit()).metadata.title,'新的标题');});
 assert.deepEqual(errors,[]);writeFileSync(`${out}/ui-checks.json`,JSON.stringify({passed:checks.length,checks,errors},null,2));console.log(JSON.stringify({passed:checks.length,failed:0}));
}catch(error){writeFileSync(`${out}/ui-checks.json`,JSON.stringify({passed:checks.length,checks,error:String(error),errors},null,2));await page.screenshot({path:`${out}/ui-failure.png`}).catch(()=>{});throw error;}finally{await browser.close();}
