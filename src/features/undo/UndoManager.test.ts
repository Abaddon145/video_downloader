import test from 'node:test';
import assert from 'node:assert/strict';
import {UndoManager} from './UndoManager.ts';
test('failed compensation keeps the operation available and concurrent undo cannot run twice',async()=>{
 const manager=new UndoManager();let fail=true,runs=0;
 manager.push({label:'移动笔记',undo:async()=>{runs++;await Promise.resolve();if(fail)throw Error('文件已变化');}});
 const one=manager.undo();assert.equal(await manager.undo(),false);
 await assert.rejects(one,/文件已变化/);assert.equal(manager.peek()?.label,'移动笔记');
 fail=false;assert.equal(await manager.undo(),true);assert.equal(manager.peek(),null);assert.equal(runs,2);
});
test('undo history is bounded and never records failed forward operations',()=>{
 const manager=new UndoManager(2);for(let i=0;i<3;i++)manager.push({label:String(i),undo:async()=>{}});
 assert.equal(manager.size,2);assert.equal(manager.peek()?.label,'2');
});
