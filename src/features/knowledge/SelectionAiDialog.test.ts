import test from 'node:test';
import assert from 'node:assert/strict';
import {applySelectionDraft,selectionMatches} from './SelectionAiDraft.ts';

test('selection AI applies only the exact raw range in its source snapshot',()=>{
 const source='A\r\n中文\r\nB';
 const selection={source,selection:'中文',from:3,to:5};
 assert.equal(applySelectionDraft(selection,'摘要',source),'A\r\n摘要\r\nB');
 assert.equal(applySelectionDraft(selection,'摘要','A\r\n改动\r\nB'),null);
 assert.equal(applySelectionDraft({...selection,from:2},'摘要',source),null);
});

test('a selection draft cannot apply after an edit or source revision',()=>{
 const expected={vaultId:'vault',path:'note.md',hash:'disk-1',text:'中文',editVersion:2};
 assert.equal(selectionMatches(expected,{...expected}),true);
 assert.equal(selectionMatches(expected,{...expected,editVersion:3}),false);
 assert.equal(selectionMatches(expected,{...expected,hash:'disk-2'}),false);
 assert.equal(selectionMatches(expected,{...expected,path:'other.md'}),false);
});
