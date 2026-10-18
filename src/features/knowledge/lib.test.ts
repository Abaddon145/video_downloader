import test from 'node:test';import assert from 'node:assert/strict';
import {MAX_OUTLINE_CHARACTERS,relativeNotePath,headings,remarkKnowledge,headingFragmentId,previewMarkdown,safeMarkdownUrl} from './lib.ts';
test('Markdown relative paths remain inside Vault, including encoded traversal',()=>{
 assert.equal(relativeNotePath('映流/课程/笔记.md','../Attachments/截图.png'),'映流/Attachments/截图.png');
 assert.equal(relativeNotePath('映流/笔记.md','../../Windows/secret.md'),null);
 for(const path of ['C:/Windows/a.md','//server/a.md','https://example.com/a.png','%2e%2e/%2e%2e/private.md','note.md:stream','javascript:alert(1)'])assert.equal(relativeNotePath('映流/笔记.md',path),null,path);
});
test('outline skips metadata and fenced code and gives duplicate headings distinct ids',()=>{
 const h=headings('---\ntitle: X\n---\n# 中文\n```md\n# Hidden\n```\n## GPU Collision\n## GPU Collision');assert.deepEqual(h.map(x=>x.id),['中文','gpu-collision','gpu-collision-1']);assert.ok(h.every(x=>x.offset>=0));
});
test('Wiki AST transform preserves code and emits guarded internal links and assets',()=>{
 const tree={type:'root',children:[{type:'paragraph',children:[{type:'text',value:'[[Folder/Note#Heading|目标]] ![[Attachments/p.png]]'}]},{type:'code',value:'[[Do not transform]]'}]};remarkKnowledge()(tree);
 const p=tree.children[0].children!;assert.equal(p[0].type,'link');assert.ok(p[0].url?.startsWith('yingliu-wiki:'));assert.equal(p.at(-1)?.type,'image');assert.equal(tree.children[1].value,'[[Do not transform]]');
});


test('heading fragments reject malformed escapes and preserve duplicate heading ids',()=>{
 assert.equal(headingFragmentId('#%E4%B8%AD%E6%96%87'),'中文');
 assert.equal(headingFragmentId('#GPU%20Collision-1'),'gpu-collision-1');
 assert.equal(headingFragmentId('#%E0%A4%A'),null);
});
test('preview hides only closed leading YAML metadata',()=>{
 assert.equal(previewMarkdown('---\ntitle: Secret\n---\n# Visible'),'# Visible');
 assert.equal(previewMarkdown('\uFEFF---\r\ntitle: Secret\r\n---\r\n# Visible'),'# Visible');
 assert.equal(previewMarkdown('---\n# Ordinary'),'---\n# Ordinary');
 assert.equal(previewMarkdown('Text\n---\nTail'),'Text\n---\nTail');
});
test('preview links allow supported protocols and reject executable and absolute paths',()=>{
 for(const value of ['https://example.com/a','http://example.com','../Note.md','#%E4%B8%AD','yingliu-wiki:Note','yingliu-asset:p.png','yingliu://video/item?t=10'])assert.equal(safeMarkdownUrl(value),value,value);
 for(const value of ['javascript:alert(1)','data:text/html,x','file:///secret','//host/a','/secret','\\secret','%2F%2Fhost/a','java%73cript:alert(1)','note%00.md','#%E0%A4%A','https://','https://example.com\n'])assert.equal(safeMarkdownUrl(value),'',value);
});

test('outline labels and preview anchors use rendered Markdown text, including video and Wiki links',()=>{
 const source='---\ntitle: X\n---\n## [00:08](yingliu://video/lesson?t=8000) **碰撞**\n## [[Folder/Note|知识]]\n## x_y &amp; `Collision`\n';
 const rows=headings(source);
 assert.deepEqual(rows.map(h=>h.text),['00:08 碰撞','知识','x_y & Collision']);
 assert.deepEqual(rows.map(h=>h.id),['0008-碰撞','知识','x_y-collision']);
 assert.equal(rows[0].offset,source.indexOf('##'));
});

test('large notes skip synchronous outline parsing without changing their content',()=>{
 const text='# 标题\n'+'x'.repeat(MAX_OUTLINE_CHARACTERS);
 assert.deepEqual(headings(text),[]);
 assert.equal(headings('# 小笔记')[0].text,'小笔记');
});
