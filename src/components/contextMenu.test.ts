import test from 'node:test';
import assert from 'node:assert/strict';
import {clampMenu,firstEnabled,nextEnabled,isEditableTarget,itemAtPath} from './contextMenuLogic.ts';

test('menu keyboard navigation skips disabled items and wraps',()=>{
 const items=[{id:'a',label:'A',disabled:true},{id:'b',label:'B'},{id:'c',label:'C',disabled:true},{id:'d',label:'D'}];
 assert.equal(firstEnabled(items),1);
 assert.equal(nextEnabled(items,1,1),3);
 assert.equal(nextEnabled(items,3,1),1);
 assert.equal(nextEnabled(items,1,-1),3);
 assert.equal(nextEnabled(items,3,1,false),3);
 assert.equal(firstEnabled([{id:'x',label:'X',disabled:true}]),-1);
 const nested=[{id:'parent',label:'Parent',children:items}];
 assert.equal(itemAtPath(nested,[0,3])?.id,'d');
 assert.equal(itemAtPath(nested,[0,9]),undefined);
});

test('menus stay inside small viewports and editable targets keep native context menus',()=>{
 assert.deepEqual(clampMenu(790,590,224,300,800,600),{x:568,y:292});
 assert.deepEqual(clampMenu(-20,-10,224,300,800,600),{x:8,y:8});
 assert.deepEqual(clampMenu(90,90,900,900,200,150),{x:8,y:8});
 assert.equal(isEditableTarget({closest:()=>({})}),true);
 assert.equal(isEditableTarget({closest:()=>null}),false);
});
