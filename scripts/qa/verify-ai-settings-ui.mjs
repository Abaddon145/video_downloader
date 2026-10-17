import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'artifacts/ai-v0.5.0';mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
try {
const context=await browser.newContext({viewport:{width:1200,height:900}});
await context.route('**/*',r=>new URL(r.request().url()).hostname==='127.0.0.1'?r.continue():r.abort());
await context.addInitScript(()=>{
 window.isTauri=true;window.calls=[];
 let key=false,settings={model:'deepseek-flash',defaultMode:'standard',whisperModel:'base',allowTranscript:true,allowImages:true,cleanTemporaryFiles:true,autoGenerateNotes:true,autoAnalyzeDownloads:false,chunkConcurrency:1};
 window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
 window.__TAURI_INTERNALS__={transformCallback:()=>1,unregisterCallback(){},convertFileSrc:p=>p,invoke:async(command,args={})=>{
 window.calls.push({command,args});
 if(command==='get_snapshot')return {settings:{downloadDir:'C:/downloads',concurrency:2,cookieMode:'none',browser:'edge',browserProfile:'',hasCookieFile:false,proxyEnabled:false,proxyUrl:'',autoCheckCoreUpdate:true,lastCoreUpdateCheck:null,cookieSummary:null,downloadPresets:[]},tasks:[],engine:{ready:true,version:'test',ffmpegVersion:'test',denoVersion:'test',error:null},notice:null,updating:false,previewing:false,coreUpdate:null};
 if(command==='get_media_settings')return null;
 if(command==='ai_get_settings')return {settings,hasApiKey:key};
 if(command==='ai_set_deepseek_key'){key=true;return true;}
 if(command==='ai_clear_deepseek_key'){key=false;return;}
 if(command==='ai_save_settings'){settings=args.settings;return {settings,hasApiKey:key};}
 if(command==='ai_test_deepseek')return '模拟连接成功，未发送视频内容';
 return 1;
 }};
});
const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5250');await page.getByRole('button',{name:'设置',exact:true}).click();
await page.getByRole('heading',{name:'AI视频分析',exact:true}).scrollIntoViewIfNeeded();
assert.equal(await page.locator('input[type=password]').count(),0);
assert.equal(await page.getByLabel('下载完成后自动分析（可能产生API费用）').isChecked(),false);
await page.getByRole('button',{name:'设置密钥',exact:true}).click();await page.getByRole('button',{name:'更换',exact:true}).waitFor();
await page.getByRole('button',{name:'测试连接',exact:true}).click();await page.getByText('模拟连接成功，未发送视频内容').waitFor();
await page.getByLabel('允许发送精选关键帧给DeepSeek').uncheck();await page.getByRole('button',{name:'保存AI设置',exact:true}).click();
await page.getByText('AI设置已保存',{exact:true}).waitFor();
const calls=await page.evaluate(()=>window.calls);assert.equal(calls.find(c=>c.command==='ai_set_deepseek_key').args.key,undefined);assert.equal(calls.filter(c=>c.command==='ai_save_settings').at(-1).args.settings.allowImages,false);
await page.getByRole('heading',{name:'AI视频分析',exact:true}).scrollIntoViewIfNeeded();await page.screenshot({path:`${out}/phase2-ai-settings.png`});
await page.setViewportSize({width:880,height:900});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);await page.screenshot({path:`${out}/phase2-ai-settings-880.png`});
assert.deepEqual(errors,[]);writeFileSync(`${out}/phase2-ui.json`,JSON.stringify({mocked:true,checks:['no web password field','native entry command has no key argument','auto analysis default off','privacy setting saved','test connection feedback','880px no horizontal overflow','no page errors'],passed:true},null,2));
} finally {await browser.close();}
