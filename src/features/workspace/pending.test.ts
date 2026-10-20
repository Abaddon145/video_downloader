import test from 'node:test';
import assert from 'node:assert/strict';
import {defaultWorkspace} from './defaults.ts';
import {readPendingWorkspace,writePendingWorkspace,acknowledgeWorkspace} from './pending.ts';
test('legacy pending workspace remains recoverable after adding preset identity and sidebar state',()=>{
 let raw:string|null=null;
 const store={getItem:()=>raw,setItem:(_key:string,next:string)=>{raw=next;},removeItem:()=>{raw=null;}};
 const old=defaultWorkspace() as Record<string,unknown>;
 delete old.activePreset;
 delete (old.appearance as Record<string,unknown>).sidebarCollapsed;
 (old.currentLayout as ReturnType<typeof defaultWorkspace>['currentLayout']).library.view='table';
 raw=JSON.stringify(old);
 const restored=readPendingWorkspace(store);
 assert.equal(restored?.currentLayout.library.view,'table');
 assert.equal(restored?.activePreset,null);
 assert.equal(restored?.appearance.sidebarCollapsed,false);
 assert.equal(raw,JSON.stringify(old));
 acknowledgeWorkspace(store,restored!);
 assert.equal(raw,null);
});
test('pending workspace survives restart and an older save acknowledgement cannot erase the latest layout',()=>{
 let value:string|null=null;
 const store={getItem:()=>value,setItem:(_key:string,next:string)=>{value=next;},removeItem:()=>{value=null;}};
 const first=defaultWorkspace(),latest=defaultWorkspace();latest.currentLayout.library.view='list';
 writePendingWorkspace(store,first);writePendingWorkspace(store,latest);acknowledgeWorkspace(store,first);
 assert.equal(readPendingWorkspace(store)?.currentLayout.library.view,'list');
 acknowledgeWorkspace(store,latest);assert.equal(readPendingWorkspace(store),null);
 for(const raw of ['{broken','{"version":999}',JSON.stringify({...latest,apiKey:'must-not-load'}),'X'.repeat(131073)]){
  value=raw;assert.equal(readPendingWorkspace(store),null);assert.equal(value,raw);
 }
});
