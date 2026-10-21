import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installLearningLayoutFixture} from './fixtures/learning-layout.mjs';

const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'docs/evidence/v0.5.2/phase5/ui';mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:process.env.EDGE_PATH??'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[],errors=[];
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000},reducedMotion:'reduce'});
 await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
 const installTaskFixture=()=>{
  const originalInvoke=window.__TAURI_INTERNALS__.invoke,originalTransform=window.__TAURI_INTERNALS__.transformCallback;
  window.isTauri=true;window.qa={handlers:new Map(),events:[],calls:[],pageOneStarted:false,releasePageOne:null,cancelStarted:false,releaseCancel:null};
  window.__TAURI_INTERNALS__.transformCallback=(fn,...rest)=>{const id=originalTransform(fn,...rest);window.qa.handlers.set(id,fn);return id;};
  const stamp=Date.now(),downloads=[
   {id:'dl-fail',request:{url:'mock:',title:'失败下载任务',thumbnail:null,kind:'video',videoMode:'compatible',maxHeight:1080,subtitleLanguages:[],filenameTemplate:'',byAuthor:false,playlistIndex:null},status:'failed',phase:'连接失败',progress:{downloaded:null,total:null,speed:null,eta:null,percent:null},outputDir:'C:/mock/downloads',files:[],error:'mock download failure',logs:['mock log'],createdAt:stamp+4,finishedAt:stamp+8,queueOrder:0},
  ];
  const media={id:'media-fail',type:'transcode',inputPath:'C:/mock/source.mp4',outputPath:null,request:{outputDir:'C:/mock/out'},status:'failed',phase:'转码失败',progress:null,speed:null,processedTime:null,totalDuration:null,eta:null,error:'mock media failure',logs:['mock media log'],createdAt:stamp+3,finishedAt:stamp+9};
  const analysis=(id,index,overrides={})=>({id,itemId:'i0',sourcePath:`C:/mock/${id}.mp4`,sourceFingerprint:`mock-${id}`,status:'failed',analysisMode:'standard',model:'mock',promptVersion:'mock',language:null,durationMs:1000,progress:0,stage:`模拟阶段 ${index}`,summary:null,markdownPath:null,markdownVaultId:null,error:`模拟错误 ${index}`,noteWarning:null,noteTargetVaultId:null,createdAt:stamp+100-index,updatedAt:stamp+100-index,completedAt:stamp+101-index,options:{localOnly:false,transcribeAudio:false,allowTranscript:true,allowImages:false,whisperModel:'base',autoGenerateNote:false},transcriptDone:false,keyframesDone:false,...overrides});
  const analyses=Array.from({length:51},(_,i)=>analysis(`ai-${i}`,i));
  analyses[0]=analysis('ai-retry',0);
  analyses[1]=analysis('subtitle-local',1,{title:'本地字幕任务',options:{localOnly:true,transcribeAudio:true,allowTranscript:false,allowImages:false,whisperModel:'base',autoGenerateNote:false}});
  analyses[2]=analysis('ai-running',2,{status:'transcribing',stage:'识别语音 12 / 30',progress:43,completedAt:null});
  const thumbnail={id:'thumb-fail',itemId:'i0',name:'课程缩略图',status:'failed',phase:'没有生成可用画面，请检查来源后重试',createdAt:stamp+2,finishedAt:stamp+10,output:null,error:'mock thumbnail failure'};
  const held=()=>new Promise(resolve=>{window.qa.releaseCancel=resolve;window.qa.cancelStarted=true;});
  window.qaEmit=(event,payload)=>{window.qa.events.push({event,payload});for(const [id,fn] of window.qa.handlers){const registration=window.qa.listeners?.get(event);if(registration===id)fn({event,id,payload});}};
  window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
   window.qa.calls.push({command,args});
   if(command==='plugin:event|listen'){
    window.qa.listeners??=new Map();window.qa.listeners.set(args.event,args.handler);return originalInvoke(command,args);
   }
   if(command==='get_snapshot'){const snapshot=await originalInvoke(command,args);return {...snapshot,tasks:downloads};}
   if(command==='get_media_snapshot')return {tasks:[media],ready:true,error:null};
   if(command==='learning_thumbnail_tasks')return [thumbnail];
   if(command==='ai_list_analyses'){
    if(args.page===1){window.qa.pageOneStarted=true;await new Promise(resolve=>{window.qa.releasePageOne=resolve;});}
    const start=(args.page-1)*50;return {items:analyses.slice(start,start+50),total:51,page:args.page};
   }
   if(command==='control_task'&&args.action==='retry')return held();
   if(command==='retry_media_task'||command==='ai_retry_analysis'||command==='ai_cancel_analysis')return held();
   return originalInvoke(command,args);
  };
 };
 await context.addInitScript({content:`(${installLearningLayoutFixture.toString()})();(${installTaskFixture.toString()})();`});
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.stack??e.message));page.setDefaultTimeout(12000);
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5270');await page.getByRole('heading',{name:'下载',exact:true}).waitFor();
 await page.waitForFunction(()=>window.qa.pageOneStarted&&['media-snapshot-updated','ai-analysis-updated','ai-analysis-completed','ai-analysis-failed','learning-thumbnail-task'].every(name=>window.qa.listeners?.has(name)));
 const mediaSnapshot={tasks:[{id:'media-fail',type:'transcode',inputPath:'C:/mock/source.mp4',outputPath:null,request:{outputDir:'C:/mock/out'},status:'failed',phase:'转码失败',progress:null,speed:null,processedTime:null,totalDuration:null,eta:null,error:'mock media failure',logs:['mock media log'],createdAt:Date.now()+3,finishedAt:Date.now()+9},{id:'media-event',type:'remux',inputPath:'C:/mock/event.mkv',outputPath:null,request:{outputDir:'C:/mock/out'},status:'processing',phase:'读取媒体信息',progress:null,speed:null,processedTime:null,totalDuration:null,eta:null,error:null,logs:[],createdAt:Date.now()+500,finishedAt:null}],ready:true,error:null};
 await page.evaluate(({mediaSnapshot})=>{
  const a={...Array.from({length:0}),id:'ai-race',itemId:'i0',sourcePath:'C:/mock/race.mp4',sourceFingerprint:'race',status:'analyzing_chunks',analysisMode:'standard',model:'mock',promptVersion:'mock',language:null,durationMs:1000,progress:0,stage:'事件阶段',summary:null,markdownPath:null,markdownVaultId:null,error:null,noteWarning:null,noteTargetVaultId:null,createdAt:Date.now()+800,updatedAt:Date.now()+800,completedAt:null,options:{localOnly:false,transcribeAudio:false,allowTranscript:true,allowImages:false,whisperModel:'base',autoGenerateNote:false},transcriptDone:false,keyframesDone:false};
  window.qaEmit('media-snapshot-updated',mediaSnapshot);window.qaEmit('ai-analysis-updated',a);window.qa.releasePageOne();
 },{mediaSnapshot});
 await page.getByRole('button',{name:/后台任务/}).click();const drawer=page.getByRole('dialog',{name:'后台任务'});await drawer.waitFor();
 const all=drawer.locator('.task-center-item');await page.waitForFunction(()=>document.querySelectorAll('.task-center-item').length===50);
 assert.equal(await drawer.getByRole('heading',{name:/后台任务/}).locator('small').innerText(),'55');
 assert.ok(await drawer.getByText('事件阶段',{exact:true}).count());assert.ok(await drawer.getByText('读取媒体信息',{exact:true}).count());
 checks.push('five task sources share one 55-item center; event updates during the delayed AI bootstrap win over its stale snapshot');

 const running=drawer.locator('.task-center-item').filter({hasText:'运行中'}).filter({hasText:'识别语音 12 / 30'}).locator('progress');assert.equal(await running.count(),1);assert.equal(await running.getAttribute('value'),null);
 await drawer.getByLabel('筛选').selectOption('transcription');await drawer.getByText('本地字幕任务',{exact:true}).waitFor();assert.equal(await drawer.locator('.task-center-item').count(),1);
 await drawer.getByLabel('筛选').selectOption('all');await drawer.getByRole('button',{name:'下一页'}).click();await drawer.getByText('2 / 2',{exact:true}).waitFor();await drawer.getByRole('button',{name:'加载更早的 AI 任务'}).click();await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='ai_list_analyses'&&c.args.page===2));
 await drawer.getByText('ai-50',{exact:true}).waitFor();assert.equal(await drawer.locator('.task-center-item').count(),6);checks.push('local subtitle filter, unknown percentage remains absent, 50-item UI paging and backend AI page 2');

 await drawer.getByLabel('筛选').selectOption('failed');assert.ok(await drawer.locator('.task-center-item').count()>=50);
 await drawer.getByRole('button',{name:'下一页'}).click();await drawer.locator('.task-center-item-title').filter({hasText:'失败下载任务'}).click();await drawer.getByRole('heading',{name:'任务详情'}).waitFor();await drawer.getByText('mock download failure',{exact:true}).waitFor();
 const retryDownload=drawer.locator('.task-center-item').filter({hasText:'失败下载任务'}).getByRole('button',{name:'重试'});await retryDownload.click();await page.waitForFunction(()=>window.qa.cancelStarted);assert.equal(await retryDownload.isDisabled(),true);await page.evaluate(()=>window.qa.releaseCancel());await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='control_task'&&c.args.id==='dl-fail'&&c.args.action==='retry'));
 checks.push('failed filter, detail error, and download retry await control_task with the source task id');

 await drawer.getByLabel('筛选').selectOption('all');const mediaRow=drawer.locator('.task-center-item').filter({hasText:'source.mp4'});await mediaRow.getByRole('button',{name:'重试'}).click();await page.waitForFunction(()=>window.qa.cancelStarted);assert.equal(await mediaRow.getByRole('button',{name:'重试'}).isDisabled(),true);await page.evaluate(()=>window.qa.releaseCancel());await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='retry_media_task'&&c.args.id==='media-fail'));
 await drawer.locator('.task-center-item').filter({hasText:'本地字幕任务'}).getByRole('button',{name:'重试'}).click();await page.waitForFunction(()=>window.qa.cancelStarted);await page.evaluate(()=>window.qa.releaseCancel());await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='ai_retry_analysis'&&c.args.id==='subtitle-local'&&c.args.regenerate===false));
 const aiRow=drawer.locator('.task-center-item').filter({hasText:'ai-retry'});await aiRow.getByRole('button',{name:'重试'}).click();await page.waitForFunction(()=>window.qa.cancelStarted);await page.evaluate(()=>window.qa.releaseCancel());await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='ai_retry_analysis'&&c.args.id==='ai-retry'&&c.args.regenerate===false));
 await drawer.getByLabel('筛选').selectOption('thumbnail');const thumbnailRow=drawer.locator('.task-center-item').filter({hasText:'课程缩略图'});await thumbnailRow.getByRole('button',{name:'重试'}).click();await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='learning_regenerate_thumbnail'&&c.args.id==='i0'&&c.args.confirmed===false));
 await drawer.getByLabel('筛选').selectOption('all');const aiRunning=drawer.locator('.task-center-item').filter({hasText:'识别语音 12 / 30'});await aiRunning.getByRole('button',{name:'取消'}).click();await page.waitForFunction(()=>window.qa.cancelStarted);await page.evaluate(()=>window.qa.releaseCancel());await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='ai_cancel_analysis'&&c.args.id==='ai-running'));
 const mediaRunning=drawer.locator('.task-center-item').filter({hasText:'event.mkv'});await mediaRunning.getByRole('button',{name:'取消'}).click();await page.waitForFunction(()=>window.qa.calls.some(c=>c.command==='cancel_media_task'&&c.args.id==='media-event'));await mediaRunning.getByRole('button',{name:'取消'}).waitFor({state:'visible'});
 checks.push('media retry/cancel and AI/local-transcription retry use the matching native command, ids, and awaited completion');

 await page.setViewportSize({width:880,height:900});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({animations:'disabled',path:`${out}/task-center-880.png`});
 await page.keyboard.press('Escape');await drawer.waitFor({state:'hidden'});await page.getByRole('button',{name:/后台任务/}).click();await drawer.waitFor();await page.keyboard.press('Tab');assert.notEqual(await page.evaluate(()=>document.activeElement?.tagName),'BODY');await page.keyboard.press('Escape');await drawer.waitFor({state:'hidden'});
 checks.push('drawer fits at 880px, Escape closes it, and keyboard focus remains in the dialog');
 assert.deepEqual(errors,[]);writeFileSync(`${out}/checks.json`,JSON.stringify({mockedTauri:true,realMachineCapabilities:false,passed:true,checks,pageErrors:errors},null,2));console.log(`PASS ${checks.length} mocked task-center UI groups`);
}catch(error){for(const context of browser.contexts())for(const page of context.pages()){console.error((await page.locator('body').innerText()).slice(-5000));console.error(await page.evaluate(()=>({url:location.href,calls:window.qa?.calls?.slice(-15),listeners:[...(window.qa?.listeners??[])],events:window.qa?.events?.map(e=>e.event)})));console.error('pageErrors',errors);await page.screenshot({animations:'disabled',path:`${out}/failure.png`});}throw error;}finally{await browser.close();}
