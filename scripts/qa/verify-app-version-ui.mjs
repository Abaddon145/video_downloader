import {createRequire} from 'node:module';import {resolve} from 'node:path';
import assert from 'node:assert/strict';
import {writeFileSync,mkdirSync,readFileSync} from 'node:fs';
const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const root=resolve('.'),out=resolve(process.env.QA_OUTPUT_DIR??'artifacts/v050/version-ui');mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true}),checks=[];
try{
 for(const fixture of [{native:false,label:'界面预览'},{native:true,version:'9.8.7',label:'v9.8.7'},{native:true,fail:true,label:'版本不可用'}]){
  const context=await browser.newContext({viewport:{width:880,height:900}});
  await context.addInitScript(f=>{if(!f.native)return;window.isTauri=true;window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};window.__TAURI_INTERNALS__={transformCallback:()=>1,unregisterCallback(){},invoke:async command=>{if(command==='plugin:app|version'){if(f.fail)throw 'unavailable';return f.version;}if(command==='get_snapshot')throw '测试仅验证版本';return 1;}};},fixture);
  const page=await context.newPage();await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5250');await page.locator('.app-version span').filter({hasText:fixture.label}).waitFor();assert.equal(await page.locator('.app-version span').innerText(),fixture.label);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));checks.push(fixture.label);await page.screenshot({path:out+'/version-'+(fixture.native?(fixture.fail?'failure':'tauri'):'preview')+'-880.png'});await context.close();
 }
 for(const path of ['package.json','src-tauri/tauri.conf.json'])assert.equal(JSON.parse(readFileSync(root+'/'+path)).version,'0.5.0');
 mkdirSync(out,{recursive:true});writeFileSync(out+'/app-version-ui.json',JSON.stringify({mocked:true,passed:true,checks},null,2));
}finally{await browser.close();}
