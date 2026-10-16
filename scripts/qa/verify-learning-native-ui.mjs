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


await context.addInitScript(()=>{
 const invoke=window.__TAURI_INTERNALS__.invoke;
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==='learning_prepare_player'){const media=await invoke(command,args);return {...media,plan:{mode:args.proxy?'transcode':'native',reason:'直接播放原文件',cached:false,format:'mp4'}};}
  if(command.startsWith('learning_native_')){
   window.testCalls.push({command,args:structuredClone(args)});
   if(command==='learning_native_start'){window.native={session:args.session,itemId:args.itemId,ready:true,position:200,duration:900,rate:1,paused:!args.autoplay,volume:.8,muted:false,ended:false,subtitles:[],subtitleId:null,error:null};return structuredClone(window.native);}
   if(command==='learning_native_control'){await new Promise(r=>setTimeout(r,20));const a=args.action;if(a.type==='pause')window.native.paused=a.paused;if(a.type==='seek')window.native.position=a.position;if(a.type==='rate')window.native.rate=a.rate;if(a.type==='volume')window.native.volume=a.volume;if(a.type==='mute')window.native.muted=a.muted;return structuredClone(window.native);}
   if(command==='learning_native_save_progress'){window.latestProgress={positionMs:window.native.position*1000,playbackRate:window.native.rate,itemId:args.itemId};return window.latestProgress;}
   return;
  }
  return invoke(command,args);
 };
});
const page=await context.newPage(),checks=[],errors=[];page.on('pageerror',e=>errors.push(String(e)));
async function check(name,fn){try{await fn();checks.push({name,passed:true});console.log('PASS '+name);}catch(e){checks.push({name,passed:false,error:String(e)});console.log('FAIL '+name+': '+e);}}
async function start(){await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5196');await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();await page.getByRole('button',{name:/播放器回归系列.*1课/}).click();await page.getByRole('button',{name:'播放 操作回归课',exact:true}).click();}
try{
 await start();
 await check('native_request_is_default',async()=>{await page.waitForFunction(()=>window.testCalls.some(c=>c.command==='learning_prepare_player'));assert.equal(await page.evaluate(()=>window.testCalls.find(c=>c.command==='learning_prepare_player').args.native),true);await page.waitForFunction(()=>window.native?.ready,null,{timeout:3000});});
 await check('stale_native_events_are_ignored',async()=>{await page.evaluate(()=>window.testEmit('learning-native-updated',{...window.native,session:'stale-session',position:800}));assert.ok(!(await page.locator('.learning-player-time').innerText()).includes('13:20'));});
 await check('native_pause_seek_rate_persist_in_order',async()=>{await page.getByRole('button',{name:'暂停课程',exact:true}).click({timeout:2000});await page.getByRole('slider',{name:'播放位置'}).fill('410');await page.getByRole('combobox',{name:'播放倍速'}).selectOption('1.5');await page.waitForFunction(()=>window.native?.rate===1.5,null,{timeout:2000});await page.evaluate(()=>window.testEmit('learning-player-hidden',{}));await page.waitForFunction(()=>window.latestProgress?.positionMs===410000&&window.latestProgress?.playbackRate===1.5,null,{timeout:2500});});
 await check('modal_hides_native_surface',async()=>{await page.evaluate(()=>{const d=document.createElement('dialog');d.id='qa-modal';document.body.append(d);d.showModal();});await page.waitForFunction(()=>window.testCalls.some(c=>c.command==='learning_native_viewport'&&!c.args.viewport.visible),null,{timeout:2000});await page.evaluate(()=>document.getElementById('qa-modal').remove());await page.waitForFunction(()=>window.testCalls.filter(c=>c.command==='learning_native_viewport').at(-1)?.args.viewport.visible===true,null,{timeout:2000});});
 await check('native_error_does_not_start_transcode',async()=>{const before=await page.evaluate(()=>window.testCalls.filter(c=>c.command==='learning_prepare_player').length);await page.evaluate(()=>window.testEmit('learning-native-updated',{...window.native,error:'原文件解码失败'}));await page.getByRole('alert').filter({hasText:'原文件解码失败'}).waitFor({timeout:2000});assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.command==='learning_prepare_player').length),before);});
 await page.screenshot({path:out+'/native-ui.png'});
 await check('no_browser_exceptions',async()=>assert.deepEqual(errors,[]));
 const failed=checks.filter(c=>!c.passed).length;writeFileSync(out+'/native-ui.json',JSON.stringify({passed:checks.length-failed,failed,checks,errors},null,2));process.exitCode=failed?1:0;
}finally{await browser.close();}
