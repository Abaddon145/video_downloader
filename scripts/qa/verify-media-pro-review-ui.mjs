import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url);
const {chromium}=require('C:/Users/14582/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const out='C:/Users/14582/Documents/ChatGPT/视频下载软件/artifacts/media-pro';mkdirSync(out,{recursive:true});
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
  if(command==='discover_media'){if(args.paths.some(p=>p.includes('scan-slow')))await new Promise(resolve=>(window.finishDiscovery||=[]).push(resolve));return args.paths;}
  if(command==='get_snapshot')return structuredClone(window.testSnapshot);
  if(command==='get_media_snapshot'){const state=structuredClone(window.testMedia);if(location.search.includes('late-snapshot'))await new Promise(resolve=>(window.finishSnapshots||=[]).push(resolve));return state;}
  if(command==='plugin:dialog|open')return args.options?.directory?'C:/output':'C:/视频 测试/fixture.mkv';
  if(command==='probe_media'){
   if(args.path.includes('slow')&&!args.path.includes('scan-slow'))await new Promise(resolve=>window.finishSlowProbe=resolve);
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
const failures=[];async function check(name,fn){try{await fn();checks.push(name);console.log(`PASS ${name}`);}catch(e){failures.push({name,error:String(e)});console.log(`FAIL ${name}: ${e}`);}}
const tool=async name=>page.getByRole('tab',{name,exact:true}).click();
const submit=async()=>{const n=await page.evaluate(()=>window.testRequests.length);await panel().getByRole('button',{name:/^(开始处理|截取当前帧)$/}).click();await page.waitForFunction(n=>window.testRequests.length===n+1,n);return page.evaluate(()=>window.testRequests.at(-1));};
const drop=async paths=>page.evaluate(paths=>window.testEmit('tauri://drag-drop',{paths,position:{x:20,y:20}}),paths);

const settle=()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
try{
 await page.goto('http://127.0.0.1:5196');await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'媒体工具',exact:true}).click();
 await page.getByRole('button',{name:'选择媒体文件',exact:true}).click();await page.getByRole('heading',{name:'fixture.mkv',exact:true}).waitFor();
 await check('late single discovery cannot replace newer selected source',async()=>{
  await drop(['C:/scan-slow-old.mkv']);await page.waitForFunction(()=>window.finishDiscovery?.length===1);
  await drop(['C:/newest.mkv']);await page.getByRole('heading',{name:'newest.mkv',exact:true}).waitFor();
  await page.evaluate(()=>window.finishDiscovery.shift()());await settle();
  assert.equal(await page.getByRole('heading',{name:'newest.mkv',exact:true}).count(),1);
 });
 await check('late batch discovery cannot replace latest batch files or busy state',async()=>{
  await drop(['C:/scan-slow-a.mkv','C:/scan-slow-b.mkv']);await page.waitForFunction(()=>window.finishDiscovery?.length===1);
  await drop(['C:/new-a.mkv','C:/new-b.mkv']);await page.waitForFunction(()=>document.querySelector('.batch-file-list')?.textContent.includes('new-a.mkv'));
  await page.evaluate(()=>window.finishDiscovery.shift()());await settle();
  assert.match(await page.locator('.batch-file-list').textContent(),/new-a\.mkv/);assert.doesNotMatch(await page.locator('.batch-file-list').textContent(),/scan-slow/);
 });
 await check('subtitle burn offers external file only while mux retains embedded text option',async()=>{
  await tool('烧录字幕');assert.equal(await panel().getByRole('button',{name:'使用内嵌字幕',exact:true}).count(),0);
  assert.match(await panel().getByRole('textbox',{name:'字幕文件',exact:true}).getAttribute('placeholder'),/请选择/);
  await tool('外挂字幕');assert.equal(await panel().getByRole('button',{name:'使用内嵌字幕',exact:true}).count(),1);
 });
 await check('all timeline axes and selection boundaries share the same coordinates',async()=>{
  await tool('视频裁剪');await page.getByLabel('开始时间',{exact:true}).fill('00:01:00.000');await page.getByLabel('结束时间',{exact:true}).fill('00:02:00.000');
  for(const width of [1200,880]){await page.setViewportSize({width,height:780});const rects=await page.locator('.timeline-editor').evaluate(e=>{
   const inputs=[...e.querySelectorAll('input')].map(i=>{const r=i.getBoundingClientRect();return {x:r.x,width:r.width};});const r=e.querySelector('.timeline-selection').getBoundingClientRect();return {inputs,selection:{x:r.x,width:r.width}};
  });for(const i of rects.inputs){assert.ok(Math.abs(i.x-rects.inputs[0].x)<1,'track start must align');assert.ok(Math.abs(i.width-rects.inputs[0].width)<1,'track width must align');}
   const track=rects.inputs[0],expected=track.x+8+.1*(track.width-16);assert.ok(Math.abs(rects.selection.x-expected)<2,`selection ${rects.selection.x} != ${expected}`);assert.ok(Math.abs(rects.selection.width-.1*(track.width-16))<2);
  }await page.locator('.visual-editor').screenshot({path:`${out}/review-timeline.png`});
 });
 writeFileSync(`${out}/review-ui-checks.json`,JSON.stringify({passed:checks.length,failed:failures.length,checks,failures,errors},null,2));assert.deepEqual(errors,[]);assert.equal(failures.length,0);console.log(JSON.stringify({passed:checks.length,failed:0}));
}finally{await browser.close();}
