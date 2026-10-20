import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installLearningLayoutFixture} from './fixtures/learning-layout.mjs';

const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'docs/evidence/v0.5.2/phase4/ui';mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:process.env.EDGE_PATH??'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[],errors=[];
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000},reducedMotion:'reduce'});
 await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
 await context.addInitScript(installLearningLayoutFixture);
 await context.addInitScript(()=>{
  window.qaNative={starts:[],controls:[],viewports:[]};
  window.qaAnalysis={id:'qa-analysis',itemId:'i0',status:'completed',analysisMode:'standard',createdAt:1,updatedAt:1,options:{localOnly:false},summary:{oneSentenceSummary:'本地模拟总结',overview:'只用于播放器 UI 验收。',chapters:[{startMs:120000,title:'模拟章节',summary:'章节说明',keywords:[],keyframeIds:[]}],sections:[],keyPoints:[],tutorialSteps:[],terms:[],actionItems:[],tags:[]}};
  window.qaNotes=[{id:'qa-note',itemId:'i0',timeMs:240000,content:'已有时间点笔记'}];
  window.qaClipboard='';
  Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:async text=>{window.qaClipboard=String(text);},readText:async()=>window.qaClipboard}});
  const original=window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
   if(command==='get_media_snapshot')return {tasks:[]};
   if(command==='ai_list_analyses')return {items:[],total:0,page:1};
   if(command==='ai_whisper_models')return [];
   if(command==='ai_model_download')return null;
   if(command==='ai_for_item')return args.itemId==='i0'?window.qaAnalysis:null;
   if(command==='learning_notes')return {notes:window.qaNotes,total:window.qaNotes.length,page:1};
   if(command==='learning_save_note'){const note={...args.input,id:`qa-note-${window.qaNotes.length+1}`};window.qaNotes.push(note);window.calls.push({command,args});return note;}
   if(command==='learning_native_start'){
    const state={session:args.session,itemId:args.itemId,ready:true,position:30,duration:900,rate:1,paused:true,volume:.8,muted:false,ended:false,subtitles:[],subtitleId:null,error:null};
    window.qaNative.state=state;window.qaNative.starts.push(structuredClone(args));return structuredClone(state);
   }
   if(command==='learning_native_control'){
    const action=args.action,state=window.qaNative.state;window.qaNative.controls.push(structuredClone(args));
    if(action.type==='seek')state.position=action.position;if(action.type==='pause')state.paused=action.paused;if(action.type==='rate')state.rate=action.rate;if(action.type==='volume')state.volume=action.volume;if(action.type==='mute')state.muted=action.muted;
    return structuredClone(state);
   }
   if(command==='learning_native_viewport'){window.qaNative.viewports.push(structuredClone(args.viewport));return;}
   if(command==='learning_native_save_progress'){return {itemId:args.itemId,positionMs:Math.round(window.qaNative.state.position*1000),durationMs:900000,completed:false,playbackRate:window.qaNative.state.rate,lastPlayedAt:1};}
   return original(command,args);
  };
 });
 const page=await context.newPage();page.on('pageerror',error=>errors.push(error.stack??error.message));page.setDefaultTimeout(10000);
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5270');
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true}).click();
 await page.getByRole('button',{name:'打开系列',exact:true}).first().click();await page.getByRole('button',{name:'播放 布局验收课程',exact:true}).click();
 const player=page.locator('.learning-player'),surface=page.locator('.learning-native-surface');await surface.waitFor();await page.waitForFunction(()=>window.qaNative.starts.length===1&&window.qaNative.state?.ready);
 await page.waitForFunction(()=>window.qaNative.viewports.some(v=>v.visible&&v.width>0));
 checks.push('native playback is mocked and starts one player session');

 await player.evaluate(el=>el.focus());await page.keyboard.press('j');await page.waitForFunction(()=>window.qaNative.state.position===25);
 await page.keyboard.press('l');await page.waitForFunction(()=>window.qaNative.state.position===30);
 const seekCount=()=>page.evaluate(()=>window.qaNative.controls.filter(c=>c.action.type==='seek').length),beforeInput=await seekCount();
 const jump=page.getByRole('textbox',{name:'精确跳转时间',exact:true});await jump.fill('00:30');await jump.press('j');await page.waitForTimeout(100);assert.equal(await seekCount(),beforeInput);
 await player.evaluate(el=>el.focus());await page.keyboard.press('Control+k');const palette=page.getByRole('dialog',{name:'命令面板',exact:true});await palette.waitFor();assert.equal(await page.evaluate(()=>window.qaNative.starts.length),1);await page.keyboard.press('Escape');await palette.waitFor({state:'hidden'});
 await page.getByRole('button',{name:'全屏播放',exact:true}).click();await page.locator('.learning-player-native-fullscreen').waitFor();assert.ok(await page.evaluate(()=>window.qaNative.controls.some(c=>c.action.type==='fullscreen'&&c.action.enabled)));
 await page.keyboard.press('Escape');await page.locator('.learning-player-native-fullscreen').waitFor({state:'hidden'});
 assert.equal(await page.evaluate(()=>window.qaNative.starts.length),1);
 checks.push('J/L seek only with player focus; editable jump input and Ctrl+K preserve the session; fullscreen reaches the mock native control');

 const timeline=page.locator('.learning-timeline'),timelineBox=await timeline.boundingBox();await page.mouse.move(timelineBox.x+timelineBox.width*.7,timelineBox.y+timelineBox.height/2);
 await page.locator('.learning-timeline-hover').waitFor();assert.equal(await page.locator('.timeline-chapter').count(),1);assert.equal(await page.locator('.timeline-note').count(),1);
 await page.locator('.timeline-chapter').click({force:true});await page.waitForFunction(()=>window.qaNative.state.position===120);
 await page.locator('.timeline-note').click({force:true});await page.waitForFunction(()=>window.qaNative.state.position===240);
 await page.getByRole('button',{name:'复制时间点',exact:true}).click();assert.equal(await page.evaluate(()=>window.qaClipboard),'[▶ 4:00](yingliu://video/i0?t=240000)');
 await page.getByRole('button',{name:'时间点笔记',exact:true}).click();const noteDialog=page.getByRole('dialog',{name:'添加时间点笔记',exact:true});await noteDialog.waitFor();assert.equal(await noteDialog.getByRole('textbox',{name:'时间点'}).inputValue(),'4:00');
 await noteDialog.getByRole('textbox',{name:'笔记内容'}).fill('通过快捷操作创建的笔记');await noteDialog.getByRole('button',{name:'保存笔记',exact:true}).click();await noteDialog.waitFor({state:'hidden'});
 assert.ok(await page.evaluate(()=>window.calls.some(c=>c.command==='learning_save_note'&&c.args.input.itemId==='i0'&&c.args.input.timeMs===240000&&c.args.input.content==='通过快捷操作创建的笔记')));
 checks.push('timeline hover and chapter/note markers seek correctly; copy time and the existing timestamped note dialog use the current position');

 const initialWidth=await player.evaluate(el=>el.getBoundingClientRect().width),size=page.getByRole('slider',{name:'学习面板大小',exact:true});await size.focus();await page.keyboard.press('End');await page.waitForFunction(()=>JSON.parse(localStorage.getItem('qa-workspace')||'null')?.currentLayout.player.panelSize===640);
 await page.setViewportSize({width:1200,height:900});await page.waitForFunction(w=>window.qaNative.viewports.some(v=>v.width<w),initialWidth-20);assert.equal(await size.inputValue(),'640');
 await page.setViewportSize({width:1440,height:1000});await page.waitForFunction(w=>window.qaNative.viewports.some(v=>v.width>=w-20),initialWidth-20);assert.equal(await page.evaluate(()=>window.qaNative.starts.length),1);
 await page.screenshot({animations:'disabled',path:`${out}/player-1440.png`});await page.setViewportSize({width:880,height:900});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({animations:'disabled',path:`${out}/player-880.png`});
 assert.equal(await page.evaluate(()=>document.documentElement.dataset.motion),'reduced');
 checks.push('native surface tracks actual container resizing while retaining panel settings and one session; reduced-motion layout fits at 880px');

 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'设置',exact:true}).click();
 await page.getByLabel('布局预设',{exact:true}).selectOption('builtin.theater');await page.getByRole('button',{name:'应用预设',exact:true}).click();
 await page.getByRole('status').filter({hasText:'已应用布局预设'}).waitFor();await page.getByRole('button',{name:'折叠主侧栏',exact:true}).click();
 await page.waitForFunction(()=>{const s=JSON.parse(localStorage.getItem('qa-workspace')||'null');return s?.activePreset==='builtin.theater'&&s?.appearance.sidebarCollapsed===true;});
 await page.reload();await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'设置',exact:true}).click();
 assert.equal(await page.locator('.app-shell').evaluate(el=>el.classList.contains('sidebar-collapsed')),true);
 await page.getByText('当前布局：剧院模式',{exact:true}).waitFor();
 checks.push('active built-in preset identity and collapsed sidebar persist after reload');

 assert.deepEqual(errors,[]);writeFileSync(`${out}/checks.json`,JSON.stringify({mockedTauri:true,passed:true,checks,pageErrors:errors},null,2));console.log(`PASS ${checks.length} mocked player/workspace UI groups`);
}catch(error){for(const context of browser.contexts())for(const page of context.pages()){console.error((await page.locator('body').innerText()).slice(-5000));console.error(await page.evaluate(()=>({url:location.href,html:document.body.innerHTML.slice(-3000),calls:window.calls?.slice(-12)})));console.error('pageErrors',errors);await page.screenshot({animations:'disabled',path:`${out}/failure.png`});}throw error;}finally{await browser.close();}
