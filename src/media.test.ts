import {mediaParentDirectory} from './media.ts';
import test from 'node:test';
import assert from 'node:assert/strict';
import {parseMediaTime,formatMediaTime,canUseOperation,createMediaRequest,isCurrentProbe,mergeMediaTasks} from './media.ts';
const info={fileName:'测试.mkv',path:'C:/视频 测试/测试.mkv',duration:600,size:10000,container:'matroska',bitrate:null,videos:[{index:0,codec:'h264',attachedPicture:false}],audios:[{index:1,codec:'aac'}],subtitles:[],metadata:{},otherStreams:0};
test('media time accepts milliseconds and rejects invalid or reversed boundaries',()=>{
 assert.equal(parseMediaTime('00:01:25.000'),85);assert.equal(parseMediaTime('02:48.500'),168.5);assert.equal(formatMediaTime(83.5),'00:01:23.500');
 for(const x of ['-1','00:60:00','00:00:60','abc',''])assert.throws(()=>parseMediaTime(x));
 assert.throws(()=>createMediaRequest(info,{operation:'trim',outputFormat:'mp4',start:'00:02:00',end:'00:01:00'},'C:/output'));
 assert.throws(()=>createMediaRequest(info,{operation:'screenshot',outputFormat:'png',start:'00:10:00'},'C:/output'));
});
test('media operation availability and all requests remain structured',()=>{
 assert.equal(canUseOperation({...info,videos:[]},'transcode'),false);assert.equal(canUseOperation({...info,audios:[]},'extractAudio'),false);
 for (const [operation,outputFormat] of [['remux','mp4'],['transcode','mp4'],['trim','mp4'],['extractAudio','mp3'],['screenshot','png']] as const){
 const r=createMediaRequest(info,{operation,outputFormat,start:'00:00:10.500',end:'00:01:10.500'},'C:/output');
 assert.equal(r.operation,operation);assert.equal(r.inputPath,info.path);assert.equal(r.audioBitrate,320);assert.equal('args' in r,false);assert.equal('command' in r,false);
 }
 assert.throws(()=>createMediaRequest(info,{operation:'transcode',outputFormat:'webm'},'C:/output'));
});
test('late probe results are ignored and task updates preserve other records',()=>{
 assert.equal(isCurrentProbe(1,2),false);assert.equal(isCurrentProbe(2,2),true);
 const a={id:'a',status:'queued'},b={id:'b',status:'processing'};
 assert.deepEqual(mergeMediaTasks([a,b],{...b,status:'completed'}),[a,{...b,status:'completed'}]);
});

test('source directory preserves normal and extended drive roots',()=>{
 assert.equal(mediaParentDirectory('C:/clip.mp4'),'C:/');
 assert.equal(mediaParentDirectory('C:\\clip.mp4'),'C:\\');
 assert.equal(mediaParentDirectory('\\\\?\\C:\\clip.mp4'),'\\\\?\\C:\\');
 assert.equal(mediaParentDirectory('C:/视频 测试/clip.mp4'),'C:/视频 测试/');
});

test('v03 editor clamps ranges, preserves frame timing and pages thumbnails at exact intervals',async()=>{
 const m=await import('./media.ts');
 assert.equal(typeof m.editorRange,'function');
 assert.deepEqual(m.editorRange(10,5,20),[10,10.001]);
 assert.deepEqual(m.editorRange(-1,30,20),[0,20]);
 assert.equal(m.thumbnailInterval(59),2);assert.equal(m.thumbnailInterval(60),5);assert.equal(m.thumbnailInterval(1801),30);
 assert.equal(m.frameStep(25),.04);assert.equal(m.frameStep(null),null);
});
