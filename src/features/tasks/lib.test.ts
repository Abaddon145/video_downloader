import test from 'node:test';
import assert from 'node:assert/strict';
import {adaptDownload,adaptMedia,adaptAnalysis,adaptThumbnail,mergeInitial,taskPage} from './lib.ts';

test('task adapters keep source IDs distinct and only expose real capabilities',()=>{
 const download=adaptDownload({id:'1',status:'processing',request:{title:'Video'},progress:{percent:null},phase:'合并中',createdAt:1,outputDir:'D:/out',files:[]} as never);
 const media=adaptMedia({id:'1',status:'failed',type:'transcode',inputPath:'D:/a.mp4',progress:null,createdAt:2,error:'bad'} as never);
 const analysis=adaptAnalysis({id:'1',status:'transcribing',title:'Video',progress:.4,stage:'识别语音',createdAt:3,updatedAt:4,options:{localOnly:true}} as never);
 assert.deepEqual([download.id,media.id,analysis.id],['download:1','media:1','ai:1']);
 assert.deepEqual(download.actions,['cancel']);
 assert.equal(download.progress,null);
 assert.deepEqual(media.actions,['retry']);
 assert.equal(analysis.kind,'ai');
 assert.equal(analysis.phase,'识别语音');
 assert.deepEqual(analysis.actions,['cancel']);
 assert.equal(analysis.progress,null,'stage estimates are not a measurable whole-job percentage');
});

test('events received during bootstrap beat stale snapshots',()=>{
 const newer={id:'1',updatedAt:10,status:'completed'};
 const older={id:'1',updatedAt:5,status:'queued'};
 assert.deepEqual(mergeInitial([older],[newer],'ai'),[newer]);
 assert.deepEqual(mergeInitial([{id:'1'}],[{id:'2'}],'media'),[{id:'1'},{id:'2'}]);
 assert.deepEqual(mergeInitial([newer,older],[],'ai'),[newer]);
});

test('page is bounded to 50 and empty results remain page one',()=>{
 const items=Array.from({length:101},(_,i)=>i);
 assert.deepEqual(taskPage(items,3),{items:[100],page:3,pages:3});
 assert.deepEqual(taskPage([],99),{items:[],page:1,pages:1});
});
test('thumbnail attempts show real completion and cannot invent pause or numeric progress',()=>{
 const task=adaptThumbnail({id:'attempt',itemId:'course',name:'本地课程',status:'needs_action',phase:'thumbnail',createdAt:10,finishedAt:20,output:null,error:'没有可用画面'});
 assert.equal(task.progress,null);assert.equal(task.transient,true);assert.deepEqual(task.actions,['retry']);assert.equal(task.itemId,'course');
});
