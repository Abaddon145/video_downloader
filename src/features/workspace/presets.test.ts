import test from 'node:test';
import assert from 'node:assert/strict';
import {defaultWorkspace} from './defaults.ts';
import {BUILTIN_WORKSPACE_PRESETS,createWorkspacePreset,updateWorkspacePreset,renameWorkspacePreset,deleteWorkspacePreset,parseWorkspaceJson,serializeWorkspaceJson} from './presets.ts';

test('ships five independent built-in layouts and custom preset CRUD protects them',()=>{
 const builtins=BUILTIN_WORKSPACE_PRESETS();assert.deepEqual(builtins.map(p=>p.id),['builtin.default','builtin.learning','builtin.focus','builtin.batch','builtin.knowledge']);
 const settings=defaultWorkspace(),layout=structuredClone(settings.currentLayout);layout.library.view='table';
 const created=createWorkspacePreset(settings,{id:'custom-1',name:'  我的布局  ',layout});assert.equal(created.presets[0].name,'我的布局');assert.equal(created.presets[0].layout.library.view,'table');
 const updated=updateWorkspacePreset(created,'custom-1',settings.currentLayout);assert.equal(updated.presets[0].layout.library.view,'cards');
 const renamed=renameWorkspacePreset(updated,'custom-1','另一布局');assert.equal(renamed.presets[0].name,'另一布局');
 assert.throws(()=>renameWorkspacePreset(renamed,'builtin.default','不应覆盖'));
 assert.throws(()=>createWorkspacePreset(renamed,{id:'custom-1',name:'重复ID',layout}));
 assert.equal(deleteWorkspacePreset(renamed,'custom-1').presets.length,0);
 assert.notEqual(builtins[0].layout,BUILTIN_WORKSPACE_PRESETS()[0].layout);
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
 ]) assert.throws(()=>parseWorkspaceJson(JSON.stringify(invalid)));
 assert.throws(()=>serializeWorkspaceJson({...settings,apiKey:'secret'} as typeof settings));
});
