import test from 'node:test';
import assert from 'node:assert/strict';
import {defaultWorkspace} from './defaults.ts';
import {BUILTIN_WORKSPACE_PRESETS,createWorkspacePreset,updateWorkspacePreset,renameWorkspacePreset,deleteWorkspacePreset,parseWorkspaceJson,serializeWorkspaceJson,applyWorkspacePreset} from './presets.ts';

test('ships six independent built-in layouts and custom preset CRUD protects them',()=>{
 const builtins=BUILTIN_WORKSPACE_PRESETS();assert.deepEqual(builtins.map(p=>p.id),['builtin.default','builtin.learning','builtin.focus','builtin.batch','builtin.knowledge','builtin.theater']);
 assert.equal(builtins[5].layout.player.layout,'theater');
 const settings=defaultWorkspace(),layout=structuredClone(settings.currentLayout);layout.library.view='table';
 const created=createWorkspacePreset(settings,{id:'custom-1',name:'  我的布局  ',layout});assert.equal(created.presets[0].name,'我的布局');assert.equal(created.presets[0].layout.library.view,'table');
 const updated=updateWorkspacePreset(created,'custom-1',settings.currentLayout);assert.equal(updated.presets[0].layout.library.view,'cards');
 const renamed=renameWorkspacePreset(updated,'custom-1','另一布局');assert.equal(renamed.presets[0].name,'另一布局');
 assert.throws(()=>renameWorkspacePreset(renamed,'builtin.default','不应覆盖'));
 assert.throws(()=>createWorkspacePreset(renamed,{id:'custom-1',name:'重复ID',layout}));
 assert.equal(deleteWorkspacePreset(renamed,'custom-1').presets.length,0);
 assert.notEqual(builtins[0].layout,BUILTIN_WORKSPACE_PRESETS()[0].layout);
});

test('applying a built-in or custom preset persists its identity and layout',()=>{
 const base=defaultWorkspace();
 const theater=applyWorkspacePreset(base,'builtin.theater');
 assert.equal(theater.activePreset,'builtin.theater');
 assert.equal(parseWorkspaceJson(serializeWorkspaceJson(theater)).currentLayout.player.layout,'theater');
 const customLayout=structuredClone(base.currentLayout);customLayout.library.view='table';
 const custom=createWorkspacePreset(base,{id:'custom-1',name:'表格',layout:customLayout});
 const applied=applyWorkspacePreset(custom,'custom-1');
 assert.equal(applied.activePreset,'custom-1');
 assert.equal(applied.currentLayout.library.view,'table');
 assert.equal(parseWorkspaceJson(serializeWorkspaceJson(applied)).activePreset,'custom-1');
 assert.throws(()=>applyWorkspacePreset(base,'custom-missing'));
});

test('workspace JSON import is bounded, strict, and round-trips only valid settings',()=>{
 const settings=defaultWorkspace(),json=serializeWorkspaceJson(settings);assert.deepEqual(parseWorkspaceJson(json),settings);
 assert.throws(()=>parseWorkspaceJson(' '.repeat(128*1024+1)),/128 KiB/);
 for(const invalid of [
  {...settings,apiKey:'secret'},
  {...settings,appearance:{...settings.appearance,unknown:true}},
  {...settings,presets:[{id:'builtin.default',name:'伪装内置',layout:settings.currentLayout}]},
  {...settings,presets:[{id:'bad',name:'非法\u0000名称',layout:settings.currentLayout}]},
  {...settings,version:2},
  {...settings,activePreset:'custom-missing'},
  {...settings,activePreset:'builtin.missing'},
  {...settings,activePreset:'bad\u0000id'},
  {...settings,activePreset:'x'.repeat(101)},
 ]) assert.throws(()=>parseWorkspaceJson(JSON.stringify(invalid)));
 assert.throws(()=>serializeWorkspaceJson({...settings,apiKey:'secret'} as typeof settings));
});
