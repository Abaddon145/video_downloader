import test from 'node:test';
import assert from 'node:assert/strict';
import {defaultWorkspace, readWorkspace, saveWorkspaceQueue, validateWorkspace} from './defaults.ts';

test('legacy version 1 workspace keeps its layout and custom presets while gaining an unset identity',()=>{
 const old=defaultWorkspace() as Record<string,unknown>;
 old.currentLayout=structuredClone(old.currentLayout);
 (old.currentLayout as ReturnType<typeof defaultWorkspace>['currentLayout']).library.view='table';
 old.presets=[{id:'custom-old',name:'旧预设',layout:structuredClone(old.currentLayout)}];
 delete old.activePreset;
 delete (old.appearance as Record<string,unknown>).sidebarCollapsed;
 const restored=readWorkspace(old);
 assert.equal(restored.currentLayout.library.view,'table');
 assert.equal(restored.presets[0].id,'custom-old');
 assert.equal(restored.activePreset,null);
 assert.equal(restored.appearance.sidebarCollapsed,false);
 assert.equal(validateWorkspace(old).activePreset,null);
});

test('workspace accepts valid table layouts and rejects future, corrupt and out of range data',()=>{
 const settings=defaultWorkspace(); settings.currentLayout.library.view='table';
 assert.equal(readWorkspace(settings).currentLayout.library.view,'table');
 for(const bad of [true,null,{...settings,version:2},{...settings,apiKey:'secret'},{...settings,appearance:{...settings.appearance,fontSize:999}}]) assert.equal(readWorkspace(bad).currentLayout.library.view,'cards');
 for(const missing of ['title','actions']) {
  const incomplete=structuredClone(settings); incomplete.currentLayout.library.columns=incomplete.currentLayout.library.columns.filter(c=>c!==missing);
  assert.equal(readWorkspace(incomplete).currentLayout.library.view,'cards');
 }
 assert.notEqual(defaultWorkspace().currentLayout.library,defaultWorkspace().currentLayout.library);
});

test('workspace save queue continues after an error and reports it',async()=>{
 const writes:number[]=[],errors:unknown[]=[];
 const queue=saveWorkspaceQueue(async value=>{writes.push(value.appearance.fontSize);if(writes.length===1)throw new Error('disk full');},error=>errors.push(error));
 const first=defaultWorkspace(),next=defaultWorkspace();next.appearance.fontSize=16;
 queue(first);queue(next);await new Promise(resolve=>setTimeout(resolve,0));
 assert.deepEqual(writes,[14,16]);assert.equal(errors.length,1);
});

test('workspace saves serialize pending changes and keep the newest layout',async()=>{
 const writes:number[]=[]; const releases:(()=>void)[]=[];
 const queue=saveWorkspaceQueue(async value=>{writes.push(value.appearance.fontSize);await new Promise<void>(resolve=>releases.push(resolve));});
 const first=defaultWorkspace();first.appearance.fontSize=14;
 const second=defaultWorkspace();second.appearance.fontSize=15;
 const third=defaultWorkspace();third.appearance.fontSize=16;
 queue(first);queue(second);queue(third);
 assert.deepEqual(writes,[14]);releases.shift()!();await new Promise(resolve=>setTimeout(resolve,0));
 assert.deepEqual(writes,[14,16]);releases.shift()!();await new Promise(resolve=>setTimeout(resolve,0));
});
