import test from 'node:test';
import assert from 'node:assert/strict';
import {effectivePlayerLayout,hoverPosition,timelineMarkers,canHideControls} from './playerInteraction.ts';
import {nativeBindings} from './nativeBindings.ts';
test('native bindings mirror current shortcuts and omit conflicting, disabled and editor commands',()=>{
 const execute=()=>{};
 const bindings=nativeBindings([{id:'player.toggle',title:'播放',category:'播放器',shortcut:'Space',scope:'player',execute},{id:'app.palette',title:'命令',category:'全局',shortcut:'Ctrl+K',execute},{id:'knowledge.save',title:'保存',category:'知识库',shortcut:'Ctrl+S',scope:'knowledge',execute},{id:'player.mute',title:'静音',category:'播放器',shortcut:'M',enabled:false,execute}]);
 assert.deepEqual(bindings,[{id:'player.toggle',key:'SPACE'},{id:'app.palette',key:'Ctrl+k'}]);
});
test('native text bindings use actual mpv characters rather than nonexistent PLUS or shifted text names',()=>{
 const execute=()=>{};
 assert.deepEqual(nativeBindings([{id:'app.palette',title:'面板',category:'全局',shortcut:'Ctrl+Shift+1',execute},{id:'learning.search',title:'查找',category:'学习',shortcut:'Ctrl+Plus',execute},{id:'player.mute',title:'静音',category:'播放',shortcut:'Shift+M',scope:'player',execute}]),[{id:'app.palette',key:'Ctrl+!'},{id:'learning.search',key:'Ctrl++'},{id:'player.mute',key:'M'}]);
});

test('player fits its container without overwriting the saved layout or panel size',()=>{
 assert.equal(effectivePlayerLayout('standard',900,400),'stacked');
 assert.equal(effectivePlayerLayout('standard',1400,400),'standard');
 assert.equal(effectivePlayerLayout('focus',400,400),'focus');
 assert.equal(effectivePlayerLayout('standard',Number.NaN,400),'stacked');
});
test('hover seeks stay finite and inside the media duration',()=>{
 assert.equal(hoverPosition(50,0,100,120),60);
 assert.equal(hoverPosition(-5,0,100,120),0);
 assert.equal(hoverPosition(200,0,100,120),120);
 assert.equal(hoverPosition(50,0,0,120),null);
 assert.equal(hoverPosition(50,0,100,Infinity),null);
});
test('timeline drops invalid markers, caps work and keeps known chapter and note positions',()=>{
 const markers=timelineMarkers([{timeMs:1000,title:'chapter',kind:'chapter'},{timeMs:-1,title:'bad',kind:'note'},{timeMs:99999,title:'late',kind:'note'},{timeMs:2000,title:'note',kind:'note'}],10);
 assert.deepEqual(markers.map(m=>m.timeMs),[1000,2000]);
 assert.equal(timelineMarkers(Array.from({length:1000},(_,i)=>({timeMs:i,title:'n',kind:'note' as const})),10).length,250);
 assert.equal(canHideControls(true,false,false,false),true);
 assert.equal(canHideControls(true,false,true,false),false);
 assert.equal(canHideControls(false,false,false,false),false);
 assert.equal(canHideControls(true,true,false,false),false);
 assert.equal(canHideControls(true,false,false,true),false);
});
