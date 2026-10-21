import {createRequire} from 'node:module';
import {mkdirSync,writeFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {installKnowledgeFixture} from './fixtures/knowledge.mjs';
import {installGeneralTemplateFixture} from './fixtures/general-template.mjs';

const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE??'playwright');
const out=process.env.QA_OUTPUT_DIR??'docs/evidence/v0.5.2/phase6/recovery-ui';
mkdirSync(out,{recursive:true});
const browser=await chromium.launch({executablePath:'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true});
const checks=[],errors=[];
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 await context.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
 await context.addInitScript(installKnowledgeFixture);await context.addInitScript(installGeneralTemplateFixture);
 await context.addInitScript(()=>{
  window.kb={configured:true,rev:1,notes:{'映流/原笔记.md':'# 原笔记\n保留正文','映流/旁观.md':'# 旁观'}};
  window.qaTrash={};window.qaReceipts={};window.qaFlags={};window.qaInitialMetadataPending=false;
  const original=window.__TAURI_INTERNALS__.invoke,view=data=>({vaultId:'vault-test',data:structuredClone(data)});
  const hash=path=>`mock:${window.kb.notes[path]}`;
  let seq=0,delayed=true;
  window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
   if(command==='knowledge_tree'&&window.qaFolders){const children={'':[{path:'映流',name:'映流',directory:true}],'映流':[{path:'映流/A',name:'A',directory:true}],'映流/A':[{path:'映流/A/B',name:'B',directory:true}],'映流/A/B':[{path:'映流/A/B/子笔记.md',name:'子笔记.md',directory:false}]};return view({items:children[args.parent]??[],total:(children[args.parent]??[]).length,page:1,skipped:0});}
   if(command==='knowledge_note_metadata'){
    const value={path:args.path,favorite:!!window.qaFlags[args.path],template:false,available:true};
    if(delayed&&args.path==='映流/原笔记.md'){
     delayed=false;window.qaInitialMetadataPending=true;
     await new Promise(resolve=>{window.qaReleaseMetadata=resolve;});
    }
    return view(value);
   }
   if(command==='knowledge_set_note_flag'){
    const old=!!window.qaFlags[args.path];if(old!==args.expected)throw '笔记标记已发生变化，请重新读取；未覆盖后续修改';
    window.qaFlags[args.path]=args.value;return view({path:args.path,favorite:args.value,template:false,available:true});
   }
   if(command==='knowledge_move_guarded'){
    if(!(args.from in window.kb.notes)||hash(args.from)!==args.expectedHash)throw '笔记已变化，不能撤销原操作，请重新读取。';
    if(args.to in window.kb.notes)throw '目标路径已有文件，当前文件保留';
    window.kb.notes[args.to]=window.kb.notes[args.from];delete window.kb.notes[args.from];
    return {vaultId:'vault-test',path:args.to,indexWarning:null};
   }
   if(command==='knowledge_trash_note'){
    if(!(args.path in window.kb.notes)||hash(args.path)!==args.expectedHash)throw '文件已被外部修改，请重新读取后再回收';
    const trashPath=`.yingliu-trash/qa-${++seq}.md`;
    window.qaTrash[trashPath]=window.kb.notes[args.path];delete window.kb.notes[args.path];
    window.qaReceipts[trashPath]={path:args.path,hash:args.expectedHash};
    return {vaultId:'vault-test',path:args.path,trashPath,hash:args.expectedHash,indexWarning:null};
   }
   if(command==='knowledge_restore_trashed_note'){
    const receipt=window.qaReceipts[args.trashPath];
    if(!receipt||receipt.path!==args.to||receipt.hash!==args.expectedHash)throw '恢复凭据无效';
    if(args.to in window.kb.notes)throw '目标路径已有文件，回收内容保留';
    if(`mock:${window.qaTrash[args.trashPath]}`!==args.expectedHash)throw '回收文件已变化，当前文件与回收内容均保留';
    window.kb.notes[args.to]=window.qaTrash[args.trashPath];delete window.qaTrash[args.trashPath];delete window.qaReceipts[args.trashPath];
    return {vaultId:'vault-test',path:args.to,indexWarning:null};
   }
   return original(command,args);
  };
 });
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(10000);
 await page.goto(process.env.QA_BASE_URL??'http://127.0.0.1:5290');
 await page.getByRole('navigation',{name:'主导航'}).getByRole('button',{name:'知识库',exact:true}).click();
 await page.getByRole('button',{name:'展开文件夹 映流',exact:true}).click();
 const note=name=>page.getByRole('button',{name:`打开笔记 ${name}`,exact:true});
 const menu=name=>page.locator('.context-menu-layer').getByRole('menuitem',{name,exact:true});
 const dialog=name=>page.getByRole('dialog',{name,exact:true});
 const state=()=>page.evaluate(()=>({notes:structuredClone(window.kb.notes),trash:structuredClone(window.qaTrash),receipts:structuredClone(window.qaReceipts)}));
 const rightAction=async(name,action)=>{await note(name).focus();await page.keyboard.press('Shift+F10');await menu(action).click();};
 const undo=async()=>{await page.locator('.feedback-toast').getByRole('button',{name:'撤销',exact:true}).click();};
 await note('原笔记.md').click();await page.getByRole('textbox',{name:'Markdown正文',exact:true}).waitFor();
 await page.waitForFunction(()=>window.qaInitialMetadataPending);
 await page.locator('.note-tools > details > summary').click();await page.getByRole('button',{name:'收藏笔记',exact:true}).click();
 await page.getByRole('button',{name:'已收藏',exact:true}).waitFor();
 await page.evaluate(()=>window.qaReleaseMetadata());
 await page.waitForTimeout(100);
 assert.equal(await page.getByRole('button',{name:'已收藏',exact:true}).count(),1);
 assert.equal(await page.evaluate(()=>window.qaFlags['映流/原笔记.md']),true);
 checks.push('late initial metadata cannot roll back favorite display after guarded toggle');

 await rightAction('原笔记.md','重命名');
 await dialog('重命名笔记').getByRole('textbox',{name:'相对路径'}).fill('映流/已改名.md');
 await dialog('重命名笔记').getByRole('button',{name:'保存位置'}).click();
 await note('已改名.md').waitFor();assert.equal((await state()).notes['映流/已改名.md'],'# 原笔记\n保留正文');
 await undo();await note('原笔记.md').waitFor();assert.equal((await state()).notes['映流/原笔记.md'],'# 原笔记\n保留正文');
 checks.push('rename and undo preserve exact note text and original path');

 await rightAction('原笔记.md','移动');
 await dialog('移动笔记').getByRole('textbox',{name:'相对路径'}).fill('映流/子目录/原笔记.md');
 await dialog('移动笔记').getByRole('button',{name:'保存位置'}).click();
 await page.waitForFunction(()=>window.kb.notes['映流/子目录/原笔记.md']);
 await undo();await page.waitForFunction(()=>window.kb.notes['映流/原笔记.md']);
 assert.equal((await state()).notes['映流/子目录/原笔记.md'],undefined);
 checks.push('guarded move and undo return note to original path');

 await rightAction('原笔记.md','移动');
 await dialog('移动笔记').getByRole('textbox',{name:'相对路径'}).fill('映流/另一个位置.md');
 await page.evaluate(()=>{window.kb.notes['映流/原笔记.md']='# 外部更新';});
 await dialog('移动笔记').getByRole('button',{name:'保存位置'}).click();
 await dialog('移动笔记').getByRole('alert').getByText(/笔记已变化/).waitFor();
 assert.equal((await state()).notes['映流/原笔记.md'],'# 外部更新');
 assert.equal((await state()).notes['映流/另一个位置.md'],undefined);
 await dialog('移动笔记').getByRole('button',{name:'取消'}).click();
 checks.push('stale source hash rejects move without touching the external edit');

 await rightAction('原笔记.md','移入回收站');
 await dialog('删除笔记').getByRole('button',{name:'移至回收目录'}).click();
 await page.waitForFunction(()=>Object.keys(window.qaTrash).length===1);
 assert.equal((await state()).notes['映流/原笔记.md'],undefined);
 const trashPath=Object.keys((await state()).trash)[0];
 await page.evaluate(()=>{window.kb.notes['映流/原笔记.md']='# 新占用文件';});
 await undo();
 await page.locator('.feedback-toast[role=alert]').getByText(/未能撤销/).waitFor();
 assert.equal((await state()).notes['映流/原笔记.md'],'# 新占用文件');
 assert.equal((await state()).trash[trashPath],'# 外部更新');
 assert.equal(await page.locator('.feedback-toast').getByRole('button',{name:'重试撤销'}).count(),1);
 await page.screenshot({animations:'disabled',path:`${out}/restore-conflict.png`});
 checks.push('occupied restore path fails visibly, keeps new file and trash, and retains retry');

 await page.evaluate(()=>{delete window.kb.notes['映流/原笔记.md'];window.qaTrash[Object.keys(window.qaTrash)[0]]='# 篡改的回收内容';});
 await page.locator('.feedback-toast').getByRole('button',{name:'重试撤销'}).click();
 await page.locator('.feedback-toast[role=alert]').getByText(/未能撤销/).waitFor();
 assert.equal((await state()).notes['映流/原笔记.md'],undefined);
 assert.equal((await state()).trash[trashPath],'# 篡改的回收内容');
 checks.push('modified trash content rejects restore and preserves the modified trash');
 await page.evaluate(()=>{window.qaTrash[Object.keys(window.qaTrash)[0]]='# 外部更新';});
 await page.locator('.feedback-toast').getByRole('button',{name:'重试撤销'}).click();
 await page.waitForFunction(()=>window.kb.notes['映流/原笔记.md']==='# 外部更新');
 assert.equal(Object.keys((await state()).trash).length,0);
 checks.push('retry with unchanged trash restores original location and consumes receipt');

 await page.keyboard.press('Control+k');const palette=page.getByRole('dialog',{name:'命令面板',exact:true});await palette.getByRole('combobox',{name:'搜索命令'}).fill('插入视频时间点');page.once('dialog',d=>d.accept('yingliu://video/course?t=12000'));await page.keyboard.press('Enter');await palette.waitFor({state:'hidden'});await page.waitForFunction(()=>window.kb.notes['映流/原笔记.md'].includes('yingliu://video/course?t=12000'));checks.push('time insertion command shares the actual source-preserving editor action and guarded autosave');
 await page.evaluate(()=>{window.qaFolders=true;});await page.getByRole('button',{name:'刷新知识库',exact:true}).click();const a=page.getByRole('button',{name:'展开文件夹 A',exact:true});await a.waitFor();await a.click({button:'right'});await menu('展开全部子文件夹').click();await note('子笔记.md').waitFor();await page.getByRole('button',{name:'收起文件夹 A',exact:true}).click({button:'right'});await menu('折叠全部子文件夹').click();await note('子笔记.md').waitFor({state:'hidden'});await page.getByRole('button',{name:'展开文件夹 A',exact:true}).waitFor();checks.push('folder context commands expand descendants lazily and collapse the subtree');

 assert.deepEqual(errors,[]);
 writeFileSync(`${out}/checks.json`,JSON.stringify({mockedTauri:true,browser:'headless Edge',passed:true,checks,pageErrors:errors},null,2));
 console.log(`PASS ${checks.length} mocked recovery UI groups`);
}catch(error){console.error('pageErrors',errors);for(const context of browser.contexts())for(const page of context.pages()){console.error((await page.locator('body').innerText()).slice(-3500));await page.screenshot({animations:'disabled',path:`${out}/failure.png`});}throw error;}finally{await browser.close();}
