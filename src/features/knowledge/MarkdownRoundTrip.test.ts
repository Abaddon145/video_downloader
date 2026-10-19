import test from 'node:test';
import assert from 'node:assert/strict';
import {fromRichText, toRichText} from './MarkdownRoundTrip.ts';

test('plain Markdown opens as editable and unchanged content remains byte-identical',()=>{
 for(const source of ['# Title\n\nA **bold** and *italic* note.', 'First paragraph.\n\nSecond paragraph.', '# 标题\n正文。']){
  const result=toRichText(source);
  assert.equal(result.mode,'rich',source);
  if(result.mode==='rich')assert.equal(fromRichText(result.doc,result),source);
 }
});

test('edited supported content produces Markdown rather than JSON or HTML',()=>{
 const result=toRichText('A **bold** note.');
 assert.equal(result.mode,'rich');
 if(result.mode!=='rich')return;
 const edited={type:'doc',content:[{type:'paragraph',content:[{type:'text',text:'Changed '},{type:'text',text:'bold',marks:[{type:'bold'}]}]}]};
 assert.equal(fromRichText(edited,result),'Changed **bold**');
});
test('bold and italic can be combined without discarding the changed text',()=>{
 const baseline=toRichText('Plain text');assert.equal(baseline.mode,'rich');if(baseline.mode!=='rich')return;
 const changed={type:'doc',content:[{type:'paragraph',content:[{type:'text',text:'组合输入',marks:[{type:'bold'},{type:'italic'}]}]}]};
 const markdown=fromRichText(changed,baseline);assert.equal(markdown,'***组合输入***');assert.equal(toRichText(markdown!).mode,'rich');
});

test('source fallback preserves CRLF, BOM, code, lists, and Obsidian syntax',()=>{
 const sources=['\uFEFF# Heading\n', '# Heading\r\nText', '```js\nalert(1)\n```', '- item\n- item', '- [ ] task', '---\ntitle: Note\n---\nText', '[[Note|alias]]', '![[Attachment.png]]', '![alt](photo.png)', 'Text ^block-id', '==highlight==', '> [!note] callout', '<script>alert(1)</script>'];
 for(const source of sources){const result=toRichText(source);assert.equal(result.mode,'source',source);if(result.mode==='source')assert.equal(result.value,source);}
});

test('unsupported rich document cannot be serialized into damaged Markdown',()=>{
 const result=toRichText('Plain text');
 assert.equal(result.mode,'rich');
 if(result.mode==='rich')assert.equal(fromRichText({type:'doc',content:[{type:'bulletList',content:[]}]},result),null);
});
