import test from 'node:test';
import assert from 'node:assert/strict';
import {EditorState} from '@codemirror/state';
import {previewLine, formatSelection,rawSelection} from './MarkdownLivePreview.ts';

test('preview ranges never rewrite source, including BOM, YAML, CRLF and unknown syntax',()=>{
 const source='\uFEFF---\r\ntitle: 笔记\r\n---\r\n# 标题\r\n**粗体** [[Other|别名]] ^block\r\n';
 const state=EditorState.create({doc:source,extensions:[EditorState.lineSeparator.of('\r\n')]});
 const heading=state.doc.line(4),body=state.doc.line(5);
 assert.equal(previewLine(heading.text,heading.from,false).kind,'heading-1');
 assert.equal(previewLine(heading.text,heading.from,true).hidden.length,0);
 assert.equal(previewLine(body.text,body.from,false).links[0]?.target,'Other');
 assert.equal(state.sliceDoc(),source);
});
test('AI selection offsets refer to the exact raw CRLF source',()=>{
 const source='A\r\n中文\r\nB';
 const state=EditorState.create({doc:source,selection:{anchor:2,head:4},extensions:[EditorState.lineSeparator.of('\r\n')]});
 assert.deepEqual(rawSelection(state),{source,selection:'中文',from:3,to:5});
});

test('formatting replaces only selected bytes and reports offsets inside inserted markup',()=>{
 const source='A 粗体\r\nB';
 const result=formatSelection(source,2,4,'bold');
 assert.deepEqual(result,{from:2,to:4,insert:'**粗体**',anchor:4,head:6});
 assert.equal(source.slice(0,result.from)+result.insert+source.slice(result.to),'A **粗体**\r\nB');
});

test('quote at a line boundary leaves the following line untouched',()=>{
 const source='first\nsecond\n';
 const result=formatSelection(source,0,6,'quote');
 assert.equal(source.slice(0,result.from)+result.insert+source.slice(result.to),'> first\nsecond\n');
});

test('CodeMirror sliceDoc keeps mixed line endings with an explicit separator',()=>{
 const source='a\r\nb\nc\r\n';
 const state=EditorState.create({doc:source,extensions:[EditorState.lineSeparator.of('\r\n')]});
 assert.equal(state.sliceDoc(),source);
});

test('unsafe link targets do not become active preview links',()=>{
 assert.equal(previewLine('[x](javascript:alert(1))',0,false).links.length,0);
 assert.equal(previewLine('[x](https://example.org)',0,false).links[0]?.target,'https://example.org');
});
