import {resolve as resolveQaOutput} from 'node:path';
import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync,readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=resolveQaOutput(process.env.QA_OUTPUT_DIR??'artifacts/learning-library');mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:process.env.EDGE_PATH??'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const context=await browser.newContext({viewport:{width:1200,height:780},reducedMotion:'reduce'});
await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
await context.addInitScript(()=>{
 window.isTauri=true;const callbacks={},listeners={};let next=1;
 window.testCalls=[];window.testRequests=[];window.testMedia={tasks:[],ready:true,error:null};window.learnItems=[{id:'i0',seriesId:'s1',title:'操作回归课',filePath:'C:/original.mp4',durationMs:900000,sortOrder:0,available:true,downloadState:'completed',progress:{completed:false,positionMs:200000,durationMs:900000,playbackRate:1,lastPlayedAt:1}}];window.learnSeries=[{id:'s1',categoryId:null,title:'播放器回归系列',totalItems:1,completedItems:0,totalDurationMs:900000,createdAt:1,updatedAt:1}];
 window.testSnapshot={settings:{downloadDir:'C:/downloads',concurrency:2,cookieMode:'none',browser:'edge',browserProfile:'',hasCookieFile:false,proxyEnabled:false,proxyUrl:'',autoCheckCoreUpdate:true,lastCoreUpdateCheck:null,cookieSummary:null,downloadPresets:[]},tasks:[],engine:{ready:true,version:'2026.10.09',ffmpegVersion:'FFmpeg test',denoVersion:'Deno',error:null},notice:null,updating:false,previewing:false,coreUpdate:null};
 window.testEmit=(event,payload)=>(listeners[event]||[]).forEach(id=>callbacks[id]?.({event,payload:structuredClone(payload)}));
 window.testInfo=path=>({path,fileName:path.split(/[\\/]/).pop(),size:1024*1024*128,duration:600,container:'matroska,webm',bitrate:1789569,videos:[{index:0,codec:'h264',profile:'High',width:1920,height:1080,fps:29.97,pixelFormat:'yuv420p',bitDepth:8,bitrate:1600000,attachedPicture:false,metadata:{title:'测试视频'}}],audios:[{index:1,codec:'aac',profile:'LC',sampleRate:48000,channels:2,channelLayout:'stereo',bitrate:192000,metadata:{language:'zho'}}],subtitles:[],otherStreams:0,metadata:{title:'离线媒体工具验收'}});
 window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
 window.__TAURI_INTERNALS__={convertFileSrc:path=>location.origin+'/qa-media.mp4?path='+encodeURIComponent(path),metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(fn){const id=next++;callbacks[id]=fn;return id;},async invoke(command,args={}){
  window.testCalls.push({command,args:structuredClone(args)});
  if(command==='plugin:event|listen'){(listeners[args.event]||=[]).push(args.handler);return args.handler;}
  if(command==='plugin:event|unlisten'){delete callbacks[args.eventId];return;}
  if(command==='learning_home')return {categories:window.learnCategories||[],series:window.learnSeries||[],total:(window.learnSeries||[]).length,page:1,continueLearning:[],settings:{autoplayNext:true,completionThreshold:.9,defaultPlaybackRate:1,restorePosition:true,autoSubtitles:true}};
  if(command==='learning_change'){const c=args.change;if(c.type==='deleteItem'){window.learnItems=window.learnItems.filter(i=>i.id!==c.id);window.deletedAt=window.testCalls.length;window.testEmit('learning-library-updated',{kind:'structure',id:c.id});return c.id;}if(c.type==='saveCategory')(window.learnCategories||=[]).push({id:'c1',name:c.category.name,description:'',icon:'BookOpen',color:'#38BDF8',sortOrder:0});if(c.type==='saveSeries')(window.learnSeries||=[]).push({...c.series,id:'s1',totalItems:0,completedItems:0,totalDurationMs:0,createdAt:Date.now(),updatedAt:Date.now()});return c.type==='saveCategory'?'c1':'s1';}
  if(command==='preview_sources')return [{url:args.urls[0],preview:{url:args.urls[0],title:'离线系列',isPlaylist:true,site:'Generic',uploader:'测试',thumbnail:null,duration:null,entries:[1,2,10].map(n=>({id:'v'+n,url:'https://example.com/'+n,title:'第'+n+'课',duration:900,thumbnail:null,playlistIndex:n})),formats:[],subtitles:[],compatible:true},error:null}];
  if(command==='learning_enqueue_downloads'){window.learnSeries[0].sourceUrl='https://example.com/playlist';window.learningEnqueued=structuredClone(args);return {tasks:[],seriesIds:['s1'],added:3,duplicates:0};}
  if(command==='learning_check_updates')return {series:window.learnSeries[0],preview:{url:'https://example.com/playlist',title:'新增',isPlaylist:true,entries:[],site:'Generic',uploader:'QA',formats:[],subtitles:[],compatible:true},newEntries:[{id:'new4',url:'https://example.com/4',title:'新增第4课',playlistIndex:4,thumbnail:null,duration:900}],template:{url:'',title:'',thumbnail:null,kind:'video',videoMode:'source',maxHeight:720,subtitleLanguages:['en'],filenameTemplate:'',byAuthor:false,playlistIndex:null}};
  if(command==='learning_notes')return {notes:window.learnNotes||[],total:(window.learnNotes||[]).length,page:1};
  if(command==='learning_save_note'){(window.learnNotes||=[]).push({...args.input,id:'n1'});return 'n1';}
  if(command==='learning_item')return window.learnItems.find(i=>i.id===args.id);
  if(command==='learning_live_progress'||command==='learning_save_progress'){const i=window.learnItems.find(i=>i.id===args.input.itemId);if(!i)throw '课程不存在';window.latestProgress=structuredClone(args.input);return i.progress;}if(command==='learning_relink'){window.learnItems[0].available=true;window.learnItems[0].filePath=args.path;window.testEmit('learning-library-updated',{kind:'relink',id:args.id});return window.learnItems[0];}if(command==='learning_delete_preview')return [window.learnItems[0].filePath];
  if(command==='learning_scan')return {paths:['C:/教程/01 基础.mp4','C:/教程/02 Emitter.mp4'],suggestedTitle:'本地教程',duplicates:0,existingSeriesId:null};
  if(command==='learning_import'){window.learnItems=['01 基础','02 Emitter'].map((title,n)=>({id:'i'+n,seriesId:'s1',title,filePath:'C:/教程/'+title+'.mp4',durationMs:900000,sortOrder:n,available:true,downloadState:'completed',progress:{completed:false,positionMs:0,durationMs:900000,playbackRate:1}}));window.learnSeries[0].totalItems=2;return {seriesId:'s1',added:2,duplicates:0,errors:[]};}
  if(command==='learning_prepare_player'&&!window.learnItems.find(i=>i.id===args.itemId)?.available)throw '文件缺失，请重新定位视频';if(command==='learning_prepare_player')return {item:window.learnItems.find(i=>i.id===args.itemId),series:window.learnSeries[0],previous:null,next:'i1',path:'C:/preview.mp4',subtitles:[],subtitleWarning:null,settings:{autoplayNext:true,completionThreshold:.9,defaultPlaybackRate:1,restorePosition:true,autoSubtitles:true}};
  if(command==='learning_series')return {series:window.learnSeries.find(s=>s.id===args.id),items:window.learnItems||[],page:1,total:(window.learnItems||[]).length};
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

const bytes=readFileSync(out+'/15分钟验收.mp4');await context.route('**/qa-media.mp4?*',route=>{const range=/^bytes=(\d+)-(\d*)$/.exec(route.request().headers()['range']??'');const start=range?Number(range[1]):0,end=range&&range[2]?Math.min(bytes.length-1,Number(range[2])):bytes.length-1;return route.fulfill({status:range?206:200,contentType:'video/mp4',headers:{'accept-ranges':'bytes','content-length':String(end-start+1),...(range?{'content-range':'bytes '+start+'-'+end+'/'+bytes.length}:{})},body:bytes.subarray(start,end+1)});});
const page=await context.newPage(),checks=[],errors=[];page.on('pageerror',e=>errors.push(String(e)));
async function start(missing=false){await page.goto((process.env.QA_BASE_URL??'http://127.0.0.1:5226'));if(missing)await page.evaluate(()=>window.learnItems[0].available=false);await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();await page.getByRole('button',{name:/播放器回归系列.*1课/}).click();await page.getByRole('button',{name:'播放 操作回归课',exact:true}).click();}
async function check(name,fn){try{await fn();checks.push({name,passed:true});console.log('PASS '+name);}catch(e){checks.push({name,passed:false,error:String(e),stack:e.stack});console.log('FAIL '+name+': '+String(e));}}
try{
 await check('relink remounts the current missing player without leaving the lesson',async()=>{await start(true);await page.getByRole('alert').filter({hasText:'文件缺失，请重新定位视频'}).waitFor();await page.getByRole('button',{name:'重新定位视频',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.learning-video video')?.readyState>=2,{},{timeout:3500});assert.ok(await page.locator('.learning-video video').evaluate(v=>!v.error));const sessions=await page.evaluate(()=>window.testCalls.filter(c=>c.command==='learning_prepare_player').map(c=>c.args.session));assert.ok(new Set(sessions).size>=2);});
 await check('paused rate and seek changes publish fresh progress before the five-second timer',async()=>{await start();await page.waitForFunction(()=>document.querySelector('.learning-video video')?.readyState>=2);await page.locator('.learning-video video').evaluate(v=>{v.pause();v.currentTime=250;v.dispatchEvent(new Event('timeupdate'));});await page.waitForTimeout(200);await page.waitForFunction(()=>window.latestProgress?.positionMs>=199000,{},{timeout:1500});await page.getByRole('combobox',{name:'播放倍速',exact:true}).selectOption('1.75');await page.waitForFunction(()=>window.latestProgress?.playbackRate===1.75,{},{timeout:2000});assert.equal(await page.evaluate(()=>window.latestProgress.playbackRate),1.75);await page.locator('.learning-video video').evaluate(v=>{v.currentTime=400;});await page.waitForFunction(()=>Math.abs(window.latestProgress?.positionMs-400000)<100,{},{timeout:2000});assert.ok(await page.evaluate(()=>Math.abs(window.latestProgress.positionMs-400000)<100));});
 await check('removing the current lesson stops playback and skips progress against the deleted row',async()=>{await start();await page.waitForFunction(()=>document.querySelector('.learning-video video')?.readyState>=2);await page.locator('.learning-video video').evaluate(v=>v.play());await page.getByRole('button',{name:'课程操作 操作回归课',exact:true}).click();await page.getByRole('button',{name:'移除课程 操作回归课',exact:true}).click();await page.getByRole('button',{name:'确认移除',exact:true}).click();await page.getByRole('region',{name:'学习播放器',exact:true}).waitFor({state:'hidden',timeout:2500});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>window.testCalls.slice(window.deletedAt).filter(c=>['learning_live_progress','learning_save_progress'].includes(c.command)).length),0);assert.ok(await page.evaluate(()=>window.testCalls.some(c=>c.command==='learning_cancel_player')));});
 assert.deepEqual(errors,[]);const failed=checks.filter(c=>!c.passed).length;writeFileSync(out+'/final-review-ui.json',JSON.stringify({passed:checks.length-failed,failed,checks,errors},null,2));console.log(JSON.stringify({passed:checks.length-failed,failed}));process.exitCode=failed?1:0;
}finally{await browser.close();}



