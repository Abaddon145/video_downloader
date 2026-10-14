import {resolve as resolveQaOutput} from 'node:path';
import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=resolveQaOutput(process.env.QA_OUTPUT_DIR??'artifacts/learning-library');mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:process.env.EDGE_PATH??'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const context=await browser.newContext({viewport:{width:1200,height:780},reducedMotion:'reduce'});
await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
await context.addInitScript(()=>{
 window.isTauri=true;window.testCancelled=new Set();const callbacks={},listeners={};let next=1;
 window.testCalls=[];window.testRequests=[];window.testMedia={tasks:[],ready:true,error:null};
 window.testSnapshot={settings:{downloadDir:'C:/downloads',concurrency:2,cookieMode:'none',browser:'edge',browserProfile:'',hasCookieFile:false,proxyEnabled:false,proxyUrl:'',autoCheckCoreUpdate:true,lastCoreUpdateCheck:null,cookieSummary:null,downloadPresets:[]},tasks:[],engine:{ready:true,version:'2026.10.09',ffmpegVersion:'FFmpeg test',denoVersion:'Deno',error:null},notice:null,updating:false,previewing:false,coreUpdate:null};
 window.testEmit=(event,payload)=>(listeners[event]||[]).forEach(id=>callbacks[id]?.({event,payload:structuredClone(payload)}));
 window.testInfo=path=>({path,fileName:path.split(/[\\/]/).pop(),size:1024*1024*128,duration:600,container:'matroska,webm',bitrate:1789569,videos:[{index:0,codec:'h264',profile:'High',width:1920,height:1080,fps:29.97,pixelFormat:'yuv420p',bitDepth:8,bitrate:1600000,attachedPicture:false,metadata:{title:'测试视频'}}],audios:[{index:1,codec:'aac',profile:'LC',sampleRate:48000,channels:2,channelLayout:'stereo',bitrate:192000,metadata:{language:'zho'}}],subtitles:[],otherStreams:0,metadata:{title:'离线媒体工具验收'}});
 window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
 window.__TAURI_INTERNALS__={convertFileSrc:()=> 'data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" width="160" height="90"/%3E',metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(fn){const id=next++;callbacks[id]=fn;return id;},async invoke(command,args={}){
  window.testCalls.push({command,args:structuredClone(args)});
  if(command==='plugin:event|listen'){(listeners[args.event]||=[]).push(args.handler);return args.handler;}
  if(command==='plugin:event|unlisten'){delete callbacks[args.eventId];return;}
  if(command==='learning_home')return {categories:window.learnCategories||[],series:window.learnSeries||[],total:(window.learnSeries||[]).length,page:1,continueLearning:[],settings:{autoplayNext:true,completionThreshold:.9,defaultPlaybackRate:1,restorePosition:true,autoSubtitles:true}};
  if(command==='learning_change'){const c=args.change;if(c.type==='saveCategory')(window.learnCategories||=[]).push({id:'c1',name:c.category.name,description:'',icon:'BookOpen',color:'#38BDF8',sortOrder:0});if(c.type==='saveSeries')(window.learnSeries||=[]).push({...c.series,id:'s1',totalItems:0,completedItems:0,totalDurationMs:0,createdAt:Date.now(),updatedAt:Date.now()});return c.type==='saveCategory'?'c1':'s1';}
  if(command==='preview_sources')return [{url:args.urls[0],preview:{url:args.urls[0],title:'离线系列',isPlaylist:true,site:'Generic',uploader:'测试',thumbnail:null,duration:null,entries:[1,2,10].map(n=>({id:'v'+n,url:'https://example.com/'+n,title:'第'+n+'课',duration:900,thumbnail:null,playlistIndex:n})),formats:[],subtitles:[],compatible:true},error:null}];
  if(command==='learning_enqueue_downloads'){window.learnSeries[0].sourceUrl='https://example.com/playlist';window.learningEnqueued=structuredClone(args);return {tasks:[],seriesIds:['s1'],added:3,duplicates:0};}
  if(command==='learning_check_updates')return {series:window.learnSeries[0],preview:{url:'https://example.com/playlist',title:'新增',isPlaylist:true,entries:[],site:'Generic',uploader:'QA',formats:[],subtitles:[],compatible:true},newEntries:[{id:'new4',url:'https://example.com/4',title:'新增第4课',playlistIndex:4,thumbnail:null,duration:900}],template:{url:'',title:'',thumbnail:null,kind:'video',videoMode:'source',maxHeight:720,subtitleLanguages:['en'],filenameTemplate:'',byAuthor:false,playlistIndex:null}};
  if(command==='learning_notes')return {notes:window.learnNotes||[],total:(window.learnNotes||[]).length,page:1};
  if(command==='learning_save_note'){(window.learnNotes||=[]).push({...args.input,id:'n1'});return 'n1';}
  if(command==='learning_item')return window.learnItems.find(i=>i.id===args.id);
  if(command==='learning_save_progress')return window.learnItems.find(i=>i.id===args.input.itemId).progress;
  if(command==='learning_scan')return {paths:['C:/教程/01 基础.mp4','C:/教程/02 Emitter.mp4'],suggestedTitle:'本地教程',duplicates:0,existingSeriesId:null};
  if(command==='learning_import'){window.learnItems=['01 基础','02 Emitter'].map((title,n)=>({id:'i'+n,seriesId:'s1',title,filePath:'C:/教程/'+title+'.mp4',durationMs:900000,sortOrder:n,available:true,downloadState:'completed',progress:{completed:false,positionMs:0,durationMs:900000,playbackRate:1}}));window.learnSeries[0].totalItems=2;return {seriesId:'s1',added:2,duplicates:0,errors:[]};}
  if(command==='learning_prepare_player')return {item:window.learnItems.find(i=>i.id===args.itemId),series:window.learnSeries[0],previous:null,next:'i1',path:'C:/preview.mp4',subtitles:[],subtitleWarning:null,settings:{autoplayNext:true,completionThreshold:.9,defaultPlaybackRate:1,restorePosition:true,autoSubtitles:true}};
  if(command==='learning_series')return {series:window.learnSeries.find(s=>s.id===args.id),items:window.learnItems||[],page:1,total:(window.learnItems||[]).length};
  if(command==='get_media_settings')return {hardwareAcceleration:'auto',afterDownload:'none',enabledAt:0,presets:[{id:'phone',name:'手机视频',operation:'compress',outputFormat:'mp4',videoCodec:'h264',audioCodec:'aac',resolution:1080,quality:'balanced',hardwareAcceleration:'auto',generateThumbnails:false}]};
  if(command==='save_media_settings')return args.settings;
  if(command==='preview_media'){if(window.testCancelled.has(args.session))throw '预览已取消';return args.path;}
  if(command==='cancel_media_preview'){window.testCancelled.add(args.session);return;}
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

const page=await context.newPage();try{await page.goto((process.env.QA_BASE_URL??'http://127.0.0.1:5226'));await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'媒体工具',exact:true}).click();await page.evaluate(()=>window.testEmit('tauri://drag-drop',{paths:['C:/测试/fixture.mkv'],position:{x:20,y:20}}));await page.getByRole('heading',{name:'fixture.mkv',exact:true}).waitFor();await page.getByRole('tab',{name:'视频裁剪',exact:true}).click();await page.locator('.video-preview video').waitFor({timeout:4000});const result=await page.evaluate(()=>{const requests=window.testCalls.filter(c=>c.command==='preview_media');return {requests:requests.map(c=>c.args.session),active:requests.at(-1).args.session,cancelled:[...window.testCancelled]};});assert.ok(result.requests.length>=2);assert.ok(!result.cancelled.includes(result.active),'StrictMode must not reuse a cancelled session');assert.equal(new Set(result.requests).size,result.requests.length);writeFileSync(`${out}/strictmode.json`,JSON.stringify(result,null,2));console.log('PASS StrictMode uses a fresh session after simulated cleanup');}finally{await browser.close();}
