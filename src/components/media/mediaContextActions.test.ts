import test from 'node:test';
import assert from 'node:assert/strict';
import {batchInputPaths,canRemoveMediaRecords,mediaContextIds} from './mediaContextActions.ts';

test('right click preserves selection only for a selected media task',()=>{
 const selected=new Set(['one','two']);
 assert.deepEqual(mediaContextIds('one',selected),['one','two']);
 assert.deepEqual(mediaContextIds('three',selected),['three']);
});

test('batch input uses playable outputs or source files, deduplicated',()=>{
 const tasks=[
  {id:'one',inputPath:'C:/src/video.mp4',outputPath:'C:/out/frame.png',status:'completed' as const},
  {id:'two',inputPath:'C:/src/video.mp4',outputPath:'C:/out/video.mp4',status:'completed' as const},
  {id:'three',inputPath:'C:/src/notes.srt',outputPath:'C:/out/notes.ass',status:'failed' as const},
 ];
 assert.deepEqual(batchInputPaths(tasks),['C:/src/video.mp4','C:/out/video.mp4']);
 assert.equal(canRemoveMediaRecords(tasks),true);
 assert.equal(canRemoveMediaRecords([...tasks,{id:'active',inputPath:'C:/src/live.mp4',outputPath:null,status:'processing'}]),false);
});
