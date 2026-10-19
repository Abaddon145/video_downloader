import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
import {installLearningLayoutFixture} from './fixtures/learning-layout.mjs';
const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=resolve(process.env.QA_OUTPUT_DIR??'artifacts/v051/layout-ui-evidence');mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 await context.route('**/*',r=>new URL(r.request().url()).hostname==='127.0.0.1'?r.continue():r.abort());
 await context.addInitScript(installLearningLayoutFixture);
 const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5260');
 const library=()=>page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'学习库',exact:true});
 await library().click();
 await page.locator('.library-series-cards').first().waitFor();
 const settings=page.locator('.library-view-settings');await settings.locator('summary').click();
 for(const view of ['list','table','cards']){
  await page.getByLabel('学习库显示方式').selectOption(view);
  assert.equal(await page.getByRole('button',{name:'打开系列',exact:true}).count(),2);
  assert.equal(await page.getByRole('button',{name:'收藏 布局验收系列 1',exact:true}).count(),1);
  if(view==='list'){
   assert.equal(await page.locator('.library-last-played').count(),2,'list rows show recent learning time');
   assert.equal(await page.locator('.library-series-list[data-density]').getAttribute('data-density'),'standard');
   await page.getByLabel('学习库列表密度').selectOption('compact');
   assert.equal(await page.locator('.library-series-list[data-density]').getAttribute('data-density'),'compact');
  }
 }
 await page.getByLabel('显示继续学习',{exact:true}).uncheck();await page.getByRole('heading',{name:'继续学习',exact:true}).waitFor({state:'hidden'});
 await page.getByLabel('系列排序',{exact:true}).selectOption('title');
 await page.waitForFunction(()=>window.calls.some(c=>c.command==='learning_home'&&c.args.sort==='title'));
 await page.getByLabel('学习库显示方式').selectOption('table');
 await page.getByLabel('分类',{exact:true}).uncheck();
 await page.getByLabel('系列名称列宽度').fill('320');
 await settings.locator('summary').click();await page.getByRole('button',{name:'收起分类',exact:true}).click();
 await page.waitForFunction(()=>JSON.parse(localStorage.getItem('qa-workspace')||'null')?.currentLayout.library.categoryCollapsed);
 await page.reload();await library().click();await page.locator('.library-series-table').waitFor();
 assert.equal(await page.getByRole('button',{name:'展开分类',exact:true}).count(),1);
 assert.equal(await page.locator('.library-series-table th').filter({hasText:/^分类$/}).count(),0);
 assert.equal(await page.locator('.library-series-table col').first().evaluate(el=>el.style.width),'320px');
 await page.screenshot({path:`${out}/library-table.png`});
 await page.getByRole('button',{name:'布局验收系列 1',exact:true}).click();
 await page.getByRole('button',{name:'播放 布局验收课程',exact:true}).click();
 await page.locator('.learning-native-surface[aria-busy=false]').waitFor();
 await page.getByRole('tab',{name:'笔记',exact:true}).click();
 const starts=await page.evaluate(()=>window.calls.filter(c=>c.command==='learning_native_start').length);
 const notes=await page.evaluate(()=>window.calls.filter(c=>c.command==='learning_notes').length);
 for(const layout of ['stacked','theater','focus','standard']){
  await page.getByLabel('播放器布局',{exact:true}).selectOption(layout);
  await page.locator(`.player-layout-${layout}`).waitFor();
  if(layout==='stacked'||layout==='theater'){
   const horizontal=page.getByRole('separator',{name:'调整学习面板大小'});await horizontal.focus();await page.keyboard.press('Home');
   const small=await page.locator('.learning-ai-content').evaluate(el=>el.getBoundingClientRect().height);
   await page.keyboard.press('End');
   const large=await page.locator('.learning-ai-content').evaluate(el=>el.getBoundingClientRect().height);
   assert.ok(large-small>300,`${layout} resizing must change the actual panel height even with short content`);
  }
 }
 await page.getByRole('button',{name:'收起学习面板',exact:true}).click();await page.getByRole('button',{name:'展开学习面板',exact:true}).click();
 assert.equal(await page.getByRole('tab',{name:'笔记',exact:true}).getAttribute('aria-selected'),'true');
 const split=page.getByRole('separator',{name:'调整学习面板大小'});await split.focus();await page.keyboard.press('End');assert.equal(await split.getAttribute('aria-valuenow'),'640');await page.keyboard.press('Home');assert.equal(await split.getAttribute('aria-valuenow'),'240');
 const box=await split.boundingBox();await page.mouse.move(box.x+box.width/2,box.y+20);await page.mouse.down();await page.mouse.move(box.x-140,box.y+20);await page.mouse.up();assert.ok(Number(await split.getAttribute('aria-valuenow'))>240);
 await page.getByRole('button',{name:'全屏播放',exact:true}).click();await page.locator('.learning-player-native-fullscreen').waitFor();await page.keyboard.press('Escape');await page.locator('.learning-player-native-fullscreen').waitFor({state:'hidden'});
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='learning_native_start').length),starts);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.command==='learning_notes').length),notes);
 await page.evaluate(()=>{const overlay=document.createElement('div');overlay.id='qa-overlay';overlay.dataset.nativeOverlay='true';document.body.append(overlay);});
 await page.waitForFunction(()=>window.calls.filter(c=>c.command==='learning_native_viewport').at(-1)?.args.viewport.visible===false);
 await page.evaluate(()=>document.querySelector('#qa-overlay').remove());await page.waitForFunction(()=>window.calls.filter(c=>c.command==='learning_native_viewport').at(-1)?.args.viewport.visible===true);
 await page.screenshot({path:`${out}/player-standard.png`});
 await page.setViewportSize({width:880,height:900});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 await page.screenshot({path:`${out}/player-880.png`});
 assert.deepEqual(errors,[]);
 writeFileSync(`${out}/checks.json`,JSON.stringify({mocked:true,passed:true,nativeStarts:starts,noteLoads:notes,checks:['three views share series actions','sort passed to backend before paging','continue toggle','columns/width/category collapse reload','four layouts retain native session and notes','panel collapse retains tab','keyboard boundaries and pointer resize','fullscreen retains native session','native overlay hide and restore','880px no overflow']},null,2));
 console.log('PASS Phase2 UI checks (mocked native bridge; actual mpv/DPI acceptance remains separate)');
}finally{await browser.close();}
