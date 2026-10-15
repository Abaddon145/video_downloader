
import {createRequire} from 'node:module';import {mkdirSync,readFileSync,writeFileSync,existsSync,copyFileSync,utimesSync,renameSync,symlinkSync,statSync} from 'node:fs';import {join,resolve,basename} from 'node:path';import {execFileSync} from 'node:child_process';import {createHash} from 'node:crypto';import assert from 'node:assert/strict';
const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=resolve(process.env.QA_OUTPUT_DIR??'artifacts/learning-playback'),fixture=JSON.parse(readFileSync(process.env.QA_FIXTURE_MANIFEST??join(out,'fixtures/matrix.json'),'utf8')),run=join(out,`playback-${Date.now()}`),data=join(run,'state');mkdirSync(data,{recursive:true});
const target=process.env.CARGO_TARGET_DIR??'src-tauri/target',app=resolve(target,'release/video-downloader.exe'),ffmpeg=resolve(target,'release/tools/ffmpeg.exe'),probe=resolve(target,'release/tools/ffprobe.exe');
const hash=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),checks=[],flow=[],sleep=ms=>new Promise(r=>setTimeout(r,ms));let browser,page,series,items,support;
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
async function waitFor(fn,timeout=120000){const end=Date.now()+timeout;while(Date.now()<end){const value=await fn();if(value)return value;await sleep(150);}throw Error('Condition timeout');}
async function check(name,fn){try{const detail=await fn();checks.push({name,passed:true,detail});console.log('PASS '+name);}catch(e){checks.push({name,passed:false,error:String(e),stack:e.stack});console.log('FAIL '+name+': '+e);}writeFileSync(join(run,'checks.json'),JSON.stringify(checks,null,2));}
async function connect(){execFileSync('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File','scripts/qa/launch-app.ps1',app,data,'9228'],{windowsHide:true});await waitFor(async()=>{try{browser=await chromium.connectOverCDP('http://127.0.0.1:9228');return true;}catch{return false;}},30000);page=browser.contexts()[0].pages().find(p=>!p.url().startsWith('devtools:'));await page.getByRole('heading',{name:'下载',exact:true}).waitFor();await waitFor(async()=>(await call('get_media_snapshot')).ready,30000);support=await page.evaluate(async()=>{const test=async codec=>{try{return (await navigator.mediaCapabilities.decodingInfo({type:'file',video:{contentType:`video/mp4; codecs="${codec}"`,width:1920,height:1080,bitrate:8000000,framerate:30}})).supported;}catch{return false;}};return {hevc:await test('hvc1.1.6.L120.B0'),hevc10:await test('hvc1.2.4.L120.B0')};});}
async function exit(){try{await call('exit_app');}catch{}await sleep(700);try{await browser?.close();}catch{}browser=null;}
async function openSeries(){await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();await page.getByRole('button',{name:/多格式播放验收.*12课/}).click();}
async function play(item){await page.getByRole('button',{name:`播放 ${item.title}`,exact:true}).click();await page.waitForFunction(()=>{const v=document.querySelector('.learning-video video');return v?.readyState>=2&&!v.error;},null,{timeout:120000});await page.waitForFunction(()=>!document.querySelector('.learning-video video').paused,null,{timeout:10000});}

try{await connect();
 await check('v4 external manual thumbnail stays visible and normalizes without losing priority',async()=>{
  const path=join(run,'legacy-manual.png');execFileSync(ffmpeg,['-hide_banner','-nostdin','-loglevel','error','-n','-f','lavfi','-i','color=c=blue:size=320x240','-frames:v','1','-threads','1',path],{windowsHide:true});const original=hash(path);
  const result=await call('learning_import',{input:{paths:[fixture.records[0].path],title:'旧版图片迁移',seriesId:null,categoryId:null}});const id=(await call('learning_series',{id:result.seriesId,page:1})).items[0].id;
  await exit();execFileSync(process.env.PYTHON_PATH??'python',['-c',`import sqlite3,sys
c=sqlite3.connect(sys.argv[1]);c.execute('UPDATE series_items SET thumbnail_path=? WHERE id=?',(sys.argv[3],sys.argv[2]))
for column in ['thumbnail_key','source_thumbnail_url','thumbnail_source']:c.execute('ALTER TABLE series_items DROP COLUMN '+column)
c.execute('UPDATE schema_version SET version=4');c.execute('PRAGMA user_version=4');c.commit();c.close()`,join(data,'learning.db'),id,path],{windowsHide:true});
  await connect();const first=(await call('learning_series',{id:result.seriesId,page:1})).items[0];assert.ok(first.thumbnailPath,'series read must retain the legacy manual image during normalization');assert.equal(first.thumbnailSource,'manual');
  const normalized=await waitFor(async()=>{const i=await call('learning_item',{id});return i.thumbnailSource==='manual'&&i.thumbnailPath!==path&&i.thumbnailPath&&i;},45000);assert.ok(normalized.thumbnailPath.includes('learning-cache'));assert.equal(hash(path),original);return {sourceImageKept:true,manualPriorityKept:true,normalized:normalized.thumbnailPath};
 });
 await exit();const failed=checks.filter(c=>!c.passed).length;writeFileSync(join(out,'migration-latest.json'),JSON.stringify({run,passed:checks.length-failed,failed,checks},null,2));console.log(JSON.stringify({run,passed:checks.length-failed,failed}));process.exitCode=failed?1:0;
}finally{if(browser)await exit();}
