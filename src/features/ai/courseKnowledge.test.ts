import test from 'node:test';
import assert from 'node:assert/strict';
import {appendKnowledge,subtitleMarkdown} from './courseKnowledge.ts';

test('subtitle quote keeps source, exact range, and safe Markdown text',()=>{
 const markdown=subtitleMarkdown('lesson/1',[{id:'segment-7',analysisId:'a',segmentIndex:7,startMs:61000,endMs:63000,text:'A [link](bad)\nsecond line'}]);
 assert.match(markdown,/yingliu:\/\/video\/lesson%2F1\?t=61000/);
 assert.match(markdown,/segment-7.*61000.*63000/);
 assert.ok(markdown.includes('> A \\[link\\]\\(bad\\)'));
 assert.match(markdown,/> second line/);
});

test('append requires the reviewed target hash and preserves text',()=>{
 assert.equal(appendKnowledge('Before','Quote','hash-a','hash-b'),null);
 assert.equal(appendKnowledge('Before','Quote','hash-a','hash-a'),'Before\n\nQuote\n');
 const original='\uFEFF---\r\ntitle: Course\r\n---\r\n\r\nBody \t\r\n';
 const appended=appendKnowledge(original,'Quote','hash-a','hash-a');
 assert.ok(appended?.startsWith(original));
 assert.equal(appended,`${original}\r\n\r\nQuote\r\n`);
});
