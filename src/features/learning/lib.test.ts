import {nextPlaybackFallback} from './lib.ts';
import test from 'node:test';import assert from 'node:assert/strict';import {seriesPercentage,resumePosition} from './lib.ts';
test('learning completion percentage handles empty and partial series',()=>{assert.equal(seriesPercentage(15,32),47);assert.equal(seriesPercentage(0,0),0);assert.equal(seriesPercentage(99,10),100);});
test('learning resume restores meaningful unfinished positions only',()=>{assert.equal(resumePosition(45000,false,true),45000);assert.equal(resumePosition(1000,false,true),0);assert.equal(resumePosition(45000,true,true),0);assert.equal(resumePosition(45000,false,false),0);});

import {seriesInput} from './lib.ts';test('series edits send writable fields without computed counters',()=>{const s={id:'s',categoryId:'c',title:'T',description:'D',coverPath:null,sourceUrl:null,favorite:true,totalItems:32,completedItems:15,sourceType:'YouTube',createdAt:1};assert.deepEqual(seriesInput(s),{id:'s',categoryId:'c',title:'T',description:'D',coverPath:null,sourceUrl:null,favorite:true});});

import {buildPlaylistImports} from './lib.ts';
test('playlist learning opt-in preserves full preview and independent frozen download options',()=>{const preview={url:'https://example.com/list',title:'教程',isPlaylist:true,entries:[{id:'v1',url:'https://example.com/1',playlistIndex:1}]};const template={url:'',title:'',thumbnail:null,kind:'video',videoMode:'source',maxHeight:720,subtitleLanguages:['en'],filenameTemplate:'',byAuthor:false,playlistIndex:null};const choice={enabled:true,title:'学习',seriesId:null,categoryId:'c',keepOrder:true};const imports=buildPlaylistImports([preview],{[preview.url]:choice},template);assert.equal(imports[0].preview.entries.length,1);assert.equal(imports[0].categoryId,'c');template.subtitleLanguages.push('zh-Hans');assert.deepEqual(imports[0].template.subtitleLanguages,['en']);assert.deepEqual(buildPlaylistImports([preview],{[preview.url]:{...choice,enabled:false}},template),[]);assert.deepEqual(buildPlaylistImports([preview],{[preview.url]:choice},{...template,kind:'audio'}),[]);});

test('playback fallback is bounded and source preferences never loop',()=>{
 assert.equal(nextPlaybackFallback('direct',0),1);assert.equal(nextPlaybackFallback('remux',1),2);assert.equal(nextPlaybackFallback('transcode',2),null);
});
