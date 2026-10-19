import test from 'node:test';
import assert from 'node:assert/strict';
import {NoteSessions} from './NoteSessions.ts';
import type {KnowledgeDocument,KnowledgeWrite} from '../../types/knowledge.ts';
const doc=(path:string,text:string,hash=text):KnowledgeDocument=>({path,text,hash,mtime:1});
const saved=(vaultId:string,d:KnowledgeDocument):KnowledgeWrite=>({vaultId,outcome:{status:'saved',document:d},indexWarning:null});
test('serial autosave acknowledges only its snapshot and preserves edits made during a write',async()=>{
 const releases:((r:KnowledgeWrite)=>void)[]=[],writes:{text:string;hash:string}[]=[];
 const s=new NoteSessions(async(v,p,text,hash)=>{writes.push({text,hash});return new Promise(r=>releases.push(r));});
 const key=s.open('v',doc('中文.md','base'));s.edit(key,'first');const flush=s.save(key);await Promise.resolve();s.edit(key,'second');
 assert.deepEqual(writes,[{text:'first',hash:'base'}]);releases.shift()!(saved('v',doc('中文.md','first','h1')));await new Promise(r=>setTimeout(r,0));
 assert.equal(s.get(key)!.draft,'second');assert.deepEqual(writes,[{text:'first',hash:'base'},{text:'second',hash:'h1'}]);
 releases.shift()!(saved('v',doc('中文.md','second','h2')));assert.equal(await flush,true);assert.equal(s.get(key)!.state,'saved');s.dispose();
});
test('Vault identity isolates same-name tabs and conflicts stop autosave without losing either version',async()=>{
 let calls=0;const s=new NoteSessions(async(v,p)=>{calls++;return {vaultId:v,outcome:{status:'conflict',document:doc(p,'external')},indexWarning:null};});
 const a=s.open('a',doc('same.md','a')),b=s.open('b',doc('same.md','b'));assert.notEqual(a,b);s.edit(a,'draft');
 assert.equal(await s.save(a),false);assert.equal(s.get(a)!.draft,'draft');assert.equal(s.get(a)!.conflict!.text,'external');assert.equal(s.get(b)!.draft,'b');
 s.edit(a,'newer');await new Promise(r=>setTimeout(r,350));assert.equal(calls,1);assert.equal(s.get(a)!.draft,'newer');assert.equal(s.close(a),false);s.dispose();
});
test('failed saves retain drafts, stable open never resets them, and moving requires drained queues',async()=>{
 let fail=true;const s=new NoteSessions(async(v,p,text)=>{if(fail)throw Error('disk full');return saved(v,doc(p,text));});
 const key=s.open('v',doc('a.md','original'));s.edit(key,'draft');assert.equal(await s.save(key),false);assert.equal(s.get(key)!.state,'error');
 s.open('v',doc('a.md','external'));assert.equal(s.get(key)!.draft,'draft');assert.throws(()=>s.move('v','a.md','b.md'));
 fail=false;assert.equal(await s.save(key),true);s.move('v','a.md','b.md');assert.equal(s.get(key),undefined);assert.equal(s.get(s.key('v','b.md'))!.draft,'draft');
 assert.equal(s.close(s.key('v','b.md')),true);s.dispose();
});
test('a failed conflict overwrite cannot publish an external hash with old document text',async()=>{
 const s=new NoteSessions(async()=>{throw Error('disk full');});const key=s.open('v',doc('note.md','old A','hashA'));
 s.edit(key,'my draft');s.refresh(key,doc('note.md','external B','hashB'));
 assert.equal(await s.save(key,'hashB'),false);assert.equal(s.get(key)!.document.hash,'hashA');assert.equal(s.get(key)!.document.text,'old A');
 s.discard(key);s.refresh(key,doc('note.md','external B','hashB'));assert.equal(s.get(key)!.draft,'external B');s.dispose();
});
