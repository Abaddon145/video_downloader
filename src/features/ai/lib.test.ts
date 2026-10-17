import test from 'node:test';
import assert from 'node:assert/strict';
import {analysisActions,acceptAnalysis,mediaTitle} from './lib.ts';
import {estimateCost} from './pricing.ts';
test('AI actions keep completed jobs distinct from interrupted work',()=>{
 assert.deepEqual(analysisActions('completed'),['regenerate','reanalyze','delete']);
 assert.ok(analysisActions('transcribing').includes('cancel'));
 assert.ok(!analysisActions('transcribing').includes('delete'));
 assert.ok(analysisActions('interrupted').includes('retry'));
 assert.deepEqual(analysisActions('completed',true),['reanalyze','delete']);
});
test('AI snapshots ignore old IDs and stale timestamps',()=>{
 const current={id:'new',updatedAt:20};
 assert.equal(acceptAnalysis(current,{id:'old',updatedAt:100}),current);
 assert.equal(acceptAnalysis(current,{id:'new',updatedAt:10}),current);
 assert.equal(acceptAnalysis(current,{id:'new',updatedAt:30}).updatedAt,30);
 assert.equal(mediaTitle('D:\\课程\\01 #Niagara.mp4'),'01 #Niagara');
});
test('AI price estimates are ranges and never hide unknown usage',()=>{
 const usage={model:'deepseek-flash',inputTokens:1000000,outputTokens:1000000,cachedTokens:0,unknownRequests:0};
 const cost=estimateCost(usage);assert.ok(cost&&cost.min>0&&cost.max>=cost.min);
 assert.equal(estimateCost({...usage,model:'unknown'}),null);
 assert.equal(estimateCost({...usage,unknownRequests:1}),null);
});

test('player AI events cannot leak a previous course or overwrite a newer analysis',async()=>{
 const {acceptCourseAnalysis}=await import('./lib.ts');
 const current={id:'new',itemId:'lesson',createdAt:20,updatedAt:30,options:{localOnly:false}};
 assert.equal(acceptCourseAnalysis(current,{...current,itemId:'old-lesson',updatedAt:100},'lesson'),current);
 assert.equal(acceptCourseAnalysis(current,{...current,id:'older',createdAt:10,updatedAt:100},'lesson'),current);
 assert.equal(acceptCourseAnalysis(current,{...current,id:'local',createdAt:50,options:{localOnly:true}},'lesson'),current);
 assert.equal(acceptCourseAnalysis(current,{...current,id:'next',createdAt:40},'lesson')?.id,'next');
});
