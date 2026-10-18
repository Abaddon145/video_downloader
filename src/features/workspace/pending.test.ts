import test from 'node:test';
import assert from 'node:assert/strict';
import {defaultWorkspace} from './defaults.ts';
import {readPendingWorkspace,writePendingWorkspace,acknowledgeWorkspace} from './pending.ts';
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
