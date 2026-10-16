import {createRequire} from 'node:module';
import {mkdirSync,readFileSync,writeFileSync,copyFileSync,renameSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {join,resolve} from 'node:path';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=resolve(process.env.QA_OUTPUT_DIR??'artifacts/native-playback'),run=join(out,`run-${Date.now()}`),data=join(run,'state');mkdirSync(data,{recursive:true});
const target=resolve(process.env.CARGO_TARGET_DIR??'src-tauri/target','release'),fixture=JSON.parse(readFileSync(process.env.QA_FIXTURE_MANIFEST,'utf8'));
const original=process.env.QA_ORIGINAL_WMV,checks=[],timings=[],errors=[],sleep=ms=>new Promise(r=>setTimeout(r,ms));let browser,page,appPid,appHwnd,items,series;
const hash=p=>createHash('sha256').update(readFileSync(p)).digest('hex');
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
async function wait(fn,ms=5000){const end=Date.now()+ms;while(Date.now()<end){const v=await fn();if(v)return v;await sleep(30);}throw Error('等待超时');}
async function check(name,fn){if(process.env.QA_CASES&&!new RegExp(process.env.QA_CASES).test(name))return;try{const detail=await fn();checks.push({name,passed:true,detail});console.log('PASS '+name);}catch(e){checks.push({name,passed:false,error:String(e),stack:e.stack});console.log('FAIL '+name+': '+e);}writeFileSync(join(run,'checks.json'),JSON.stringify(checks,null,2));}
function windows(name,freeze=false){return JSON.parse(execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/native-window-check.ps1','-ApplicationPid',String(appPid),'-WindowHandle',String(appHwnd),...(name?['-Screenshot',join(run,name+'.png')]:[]),...(freeze?['-Freeze']:[])],{encoding:'utf8',windowsHide:true}));}
async function launch(){const p=JSON.parse(execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/launch-app.ps1',join(target,'video-downloader.exe'),data,'9229'],{encoding:'utf8',windowsHide:true}));appPid=p.pid??p.processId??p;await wait(async()=>{try{browser=await chromium.connectOverCDP('http://127.0.0.1:9229');return true;}catch{return false;}},30000);page=browser.contexts()[0].pages().find(p=>!p.url().startsWith('devtools:'));page.on('pageerror',e=>errors.push(String(e)));await page.getByRole('heading',{name:'下载',exact:true}).waitFor();await wait(async()=>(await call('get_media_snapshot')).ready,30000);appHwnd=Number(execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Process -Id ${appPid}).MainWindowHandle.ToInt64()`],{encoding:'utf8',windowsHide:true}));assert.ok(appHwnd>0);
 await page.evaluate(async()=>{
  window.nativeStates={};
  const handler=window.__TAURI_INTERNALS__.transformCallback(e=>{window.nativeStates[e.payload.session]=e.payload;});
  await window.__TAURI_INTERNALS__.invoke('plugin:event|listen',{event:'learning-native-updated',target:{kind:'Any'},handler});
 });
}
async function exit(){try{await call('exit_app');}catch{}await sleep(700);try{await browser?.close();}catch{}browser=null;}
const viewport={left:250,top:150,width:500,height:280,visible:true};
async function start(item,session,autoplay=true){assert.ok(item,'fixture item exists');const begin=performance.now();const media=await call('learning_prepare_player',{itemId:item.id,session,native:true,proxy:false});assert.equal(media.plan.mode,'native');const s=await call('learning_native_start',{itemId:item.id,session,viewport,autoplay});assert.equal(s.ready,true);const ms=Math.round(performance.now()-begin);assert.ok(ms<5000,`startup ${ms}ms`);return {s,ms};}
try{
 await launch();
 const paths=[...fixture.records.map(r=>r.path),original];const imported=await call('learning_import',{input:{paths,title:'原文件直接播放验收',categoryId:null,seriesId:null}});series=imported.seriesId;items=(await call('learning_series',{id:series,page:1})).items;
 await check('twelve codecs start within five seconds, seek to 80%, and never enqueue conversion',async()=>{
  await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();await page.getByRole('button',{name:/原文件直接播放验收.*13课/}).click();
  for(let i=0;i<fixture.records.length;i++){
   const record=fixture.records[i],item=items.find(x=>x.title===record.name.replace(/\.[^.]+$/, ''));assert.ok(item);
   const begin=performance.now();
   await page.getByRole('button',{name:`播放 ${item.title}`,exact:true}).click();
   const state=await wait(()=>page.evaluate(id=>Object.values(window.nativeStates).findLast(s=>s.itemId===id&&s.ready&&!s.error)??null,item.id),5000);
   const s=state,session=s.session,ms=Math.round(performance.now()-begin);assert.ok(ms<5000);
await call('learning_native_control',{session,action:{type:'seek',position:s.duration*.8}});await call('learning_native_control',{session,action:{type:'rate',rate:1.5}});const after=await call('learning_native_control',{session,action:{type:'pause',paused:true}});assert.ok(Math.abs(after.position-s.duration*.8)<1);assert.equal(after.rate,1.5);timings.push({name:record.name,ms,subtitles:s.subtitles.length});await call('learning_cancel_player',{session});}
  await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'下载',exact:true}).click();
  assert.equal((await call('get_media_snapshot')).tasks.length,0);return timings;
 });
 const long=items.find(x=>x.title==='wmv3-original');assert.ok(long,'long WMV imported');
 await check('157-second original WMV starts immediately and renders in an owned child window',async()=>{const {s,ms}=await start(long,'original-wmv');await sleep(300);const w=windows('native-original');assert.ok(w.children.some(c=>c.owned&&c.visible&&c.width>300));assert.ok(w.children.some(c=>c.name.toLowerCase().includes('mpv')&&c.visible));assert.equal(w.processes.length,1);await call('learning_cancel_player',{session:'original-wmv'});return {ms,windows:w};});
 await check('embedded and local subtitles, notes, ordered pause and progress survive restart',async()=>{
  const item=items.find(x=>x.title.includes('02-MKV'));await start(item,'subtitles',false);await sleep(150);const s=await call('learning_native_control',{session:'subtitles',action:{type:'pause',paused:true}});assert.ok(s.subtitles.length);await call('learning_native_control',{session:'subtitles',action:{type:'subtitle',id:s.subtitles[0].id}});
  const sub=join(run,'中文 字幕.srt');writeFileSync(sub,'1\n00:00:00,000 --> 00:01:00,000\n原生字幕验收\n');await call('learning_native_control',{session:'subtitles',action:{type:'loadSubtitle',path:sub}});await sleep(100);await call('learning_native_control',{session:'subtitles',action:{type:'seek',position:10}});await call('learning_native_control',{session:'subtitles',action:{type:'rate',rate:1.5}});const p=await call('learning_native_save_progress',{session:'subtitles'});assert.ok(p.positionMs>=9900);assert.equal(p.playbackRate,1.5);await call('learning_save_note',{input:{itemId:item.id,id:null,timeMs:10000,content:'原生播放笔记保留'}});await call('learning_cancel_player',{session:'subtitles'});await exit();await launch();const restored=await call('learning_item',{id:item.id});assert.ok(restored.progress.positionMs>=9900);assert.equal((await call('learning_notes',{itemId:item.id,page:1})).notes[0].content,'原生播放笔记保留');return restored.progress;
 });
 await check('late cancellation cannot kill a newer native session',async()=>{await page.evaluate(id=>{window.pendingNative=window.__TAURI_INTERNALS__.invoke('learning_native_start',{itemId:id,session:'race-old',viewport:{left:250,top:150,width:500,height:280,visible:true},autoplay:true}).catch(e=>String(e));},long.id);await call('learning_cancel_player',{session:'race-old'});await start(long,'race-new');await call('learning_cancel_player',{session:'race-old'});const s=await call('learning_native_control',{session:'race-new',action:{type:'pause',paused:true}});assert.equal(s.ready,true);await page.evaluate(()=>window.pendingNative);assert.equal(windows().processes.length,1);await call('learning_cancel_player',{session:'race-new'});await sleep(150);assert.equal(windows().processes.length,0);});
 await check('actual UI uses native controls, hides under notes, and enters/exits fullscreen',async()=>{
  await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();await page.getByRole('button',{name:/原文件直接播放验收.*13课/}).click();const t=performance.now();await page.getByRole('button',{name:`播放 ${long.title}`,exact:true}).click();await page.locator('.learning-native-surface[aria-busy="false"]').waitFor({timeout:5000});const ms=Math.round(performance.now()-t);await page.locator('.learning-video').scrollIntoViewIfNeeded();await sleep(200);const w=windows('native-ui');assert.ok(w.children.some(c=>c.owned&&c.visible));
  await page.getByRole('button',{name:'暂停课程',exact:true}).click();await page.getByRole('slider',{name:'播放位置'}).fill('80');await page.getByRole('combobox',{name:'播放倍速'}).selectOption('1.5');await page.getByRole('button',{name:'添加笔记',exact:true}).click();await page.getByRole('dialog').waitFor();assert.ok(windows('native-notes').children.filter(c=>c.owned).every(c=>!c.visible));await page.getByRole('button',{name:'取消',exact:true}).click();
  await page.getByRole('button',{name:'全屏播放',exact:true}).click();await page.locator('.learning-player-native-fullscreen').waitFor();await sleep(200);assert.ok(windows('native-fullscreen').children.some(c=>c.owned&&c.visible));await page.getByRole('button',{name:'全屏播放',exact:true}).click();await page.locator('.learning-player-native-fullscreen').waitFor({state:'hidden'});
  execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/window-check.ps1','-ApplicationPid',String(appPid),'-WindowHandle',String(appHwnd),'-Action','resize','-Width','880','-Height','650'],{windowsHide:true});await page.locator('.learning-video').scrollIntoViewIfNeeded();await sleep(200);assert.ok(windows('native-narrow').children.some(c=>c.owned&&c.visible));
  await page.getByRole('region',{name:'学习播放器',exact:true}).focus();await page.keyboard.press('Space');await page.getByRole('button',{name:'暂停课程',exact:true}).waitFor({timeout:2000});await page.keyboard.press('Space');await page.getByRole('button',{name:'播放课程',exact:true}).waitFor({timeout:2000});
  execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/window-check.ps1','-ApplicationPid',String(appPid),'-WindowHandle',String(appHwnd),'-Action','resize','-Width','1200','-Height','780'],{windowsHide:true});
  await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'下载',exact:true}).click();await sleep(300);assert.equal(windows().processes.length,0);return {ms};
 });
 await check('failed native playback retires its owned window without navigation',async()=>{
  await start(long,'crashed');
  execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/native-window-check.ps1','-ApplicationPid',String(appPid),'-StopEngine'],{windowsHide:true});
  await sleep(900);
  assert.equal(windows().children.filter(c=>c.owned).length,0);
  await assert.rejects(()=>call('learning_native_control',{session:'crashed',action:{type:'pause',paused:true}}),/会话已关闭/);
 });
 await check('unresponsive IPC times out and terminates only the owned process',async()=>{await start(long,'hung');windows(null,true);const t=performance.now();await assert.rejects(()=>call('learning_native_control',{session:'hung',action:{type:'pause',paused:true}}));const ms=Math.round(performance.now()-t);assert.ok(ms<4500);await call('learning_cancel_player',{session:'hung'});await sleep(200);assert.equal(windows().processes.length,0);return {ms};});
 await check('missing and corrupt bundled engine fail without starting conversion',async()=>{const exe=join(target,'tools/mpv.exe'),backup=exe+'.qa-backup';renameSync(exe,backup);try{await assert.rejects(()=>start(long,'missing'));writeFileSync(exe,'invalid engine');await assert.rejects(()=>start(long,'corrupt'));}finally{renameSync(backup,exe);}assert.equal((await call('get_media_snapshot')).tasks.length,0);assert.equal(windows().processes.length,0);});
 await check('manual compatibility conversion remains available and never overwrites the source',async()=>{
  const item=items.find(x=>x.title.includes('06-WMV')),session='manual-compatible';
  const media=await call('learning_prepare_player',{itemId:item.id,session,native:true,proxy:true,fallback:2});assert.equal(media.plan.mode,'transcode');assert.ok(media.path.toLowerCase().endsWith('.mp4'));assert.notEqual(media.path,item.filePath);await call('learning_cancel_player',{session});
 });
 await check('hide and immediate exit pause and save actual native progress',async()=>{
  await start(long,'hidden-progress');await call('learning_native_control',{session:'hidden-progress',action:{type:'seek',position:70}});await call('learning_native_control',{session:'hidden-progress',action:{type:'rate',rate:1.5}});
  execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/window-check.ps1','-ApplicationPid',String(appPid),'-WindowHandle',String(appHwnd),'-Action','close'],{windowsHide:true});
  const saved=(await call('learning_item',{id:long.id})).progress;assert.ok(saved.positionMs>=70000);assert.equal(saved.playbackRate,1.5);
  const paused=await call('learning_native_control',{session:'hidden-progress',action:{type:'volume',volume:.7}});assert.equal(paused.paused,true);
  execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/window-check.ps1','-ApplicationPid',String(appPid),'-WindowHandle',String(appHwnd),'-Action','show'],{windowsHide:true});
  await call('learning_native_control',{session:'hidden-progress',action:{type:'seek',position:85}});await exit();await launch();const restored=(await call('learning_item',{id:long.id})).progress;assert.ok(restored.positionMs>=85000);assert.equal(restored.playbackRate,1.5);return restored;
 });
 await check('damaged registered media fails in bounded time without orphaned windows',async()=>{
  const damaged=join(run,'损坏输入.wmv');copyFileSync(original,damaged);const imported=await call('learning_import',{input:{paths:[damaged],title:'损坏文件恢复',categoryId:null,seriesId:null}});const item=(await call('learning_series',{id:imported.seriesId,page:1})).items[0];writeFileSync(damaged,'not a video');const t=performance.now();await assert.rejects(()=>call('learning_native_start',{itemId:item.id,session:'damaged',viewport,autoplay:true}));assert.ok(performance.now()-t<6000);await sleep(200);assert.equal(windows().children.filter(c=>c.owned).length,0);assert.equal(windows().processes.length,0);
 });
 await check('all original fixtures remain byte-identical',async()=>{for(const record of fixture.records)assert.equal(hash(record.path),record.sha256);});
 await check('no JavaScript exceptions',async()=>assert.deepEqual(errors,[]));
}finally{
 if(browser)await exit();const failed=checks.filter(c=>!c.passed).length;writeFileSync(join(out,'native-latest.json'),JSON.stringify({run,passed:checks.length-failed,failed,checks,timings,errors},null,2));console.log(JSON.stringify({run,failed}));process.exitCode=failed?1:0;
}
