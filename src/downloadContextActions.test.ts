import test from 'node:test';
import assert from 'node:assert/strict';
import {contextTaskIds} from './downloadContextActions.ts';

test('a context menu uses selected tasks only when the clicked row is selected',()=>{
 const selected=new Set(['a','b']);
 assert.deepEqual(contextTaskIds('a',selected),['a','b']);
 assert.deepEqual(contextTaskIds('c',selected),['c']);
 selected.add('d');
 assert.deepEqual(contextTaskIds('a',selected),['a','b','d']);
});
