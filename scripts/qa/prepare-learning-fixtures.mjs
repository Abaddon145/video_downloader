import {execFileSync} from 'node:child_process';import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs';import {resolve,join} from 'node:path';import {createHash} from 'node:crypto';
const root=resolve(process.env.QA_FIXTURE_DIR??'artifacts/learning-playback/fixtures'),out=join(root,'matrix');mkdirSync(out,{recursive:true});
const tools=resolve(process.env.CARGO_TARGET_DIR??'src-tauri/target','release/tools');
const ff=(args)=>execFileSync(join(tools,'ffmpeg.exe'),['-hide_banner','-nostdin','-loglevel','error','-n',...args],{windowsHide:true,timeout:120000});
const base=join(out,'01-MP4-H264-AAC.mp4');if(!existsSync(base))ff(['-f','lavfi','-i','testsrc2=size=320x180:rate=10','-f','lavfi','-i','sine=frequency=440:sample_rate=48000','-t','20','-c:v','libx264','-preset','ultrafast','-pix_fmt','yuv420p','-threads','1','-c:a','aac','-b:a','96k',base]);
const srt=join(out,'subtitle.srt');writeFileSync(srt,'1\n00:00:01,000 --> 00:00:15,000\n多格式兼容播放验收\n');
const definitions=[
 ['02-MKV-H264-AAC.mkv',['-i',base,'-i',srt,'-map','0','-map','1:0','-c','copy','-c:s','srt']],
 ['03-MKV-HEVC-AAC.mkv',['-i',base,'-c:v','libx265','-preset','ultrafast','-crf','30','-x265-params','pools=1:frame-threads=1:log-level=error','-c:a','copy']],
 ['04-MOV-H264.mov',['-i',base,'-c','copy']],
 ['05-AVI-MPEG4.avi',['-i',base,'-c:v','mpeg4','-q:v','5','-threads','1','-c:a','libmp3lame','-b:a','128k']],
 ['06-WMV-WMV3-WMA.wmv',['-i',join(root,'wmv3-original.wmv'),'-t','20','-map','0:v:0','-map','0:a:0','-c','copy']],
 ['07-WMV-VC1-WMA.wmv',['-i',join(root,'vc1-audio-original.wmv'),'-t','20','-map','0:v:0','-map','0:a:0','-c','copy']],
 ['08-MPEG-MPEG2.mpg',['-i',base,'-c:v','mpeg2video','-q:v','4','-threads','1','-c:a','mp2','-b:a','192k','-ar','48000']],
 ['09-TS-H264.ts',['-i',base,'-c','copy','-f','mpegts']],
 ['10-M2TS-H264.m2ts',['-i',base,'-c','copy','-f','mpegts','-mpegts_m2ts_mode','1']],
 ['11-WebM-VP9-Opus.webm',['-i',base,'-c:v','libvpx-vp9','-deadline','realtime','-cpu-used','8','-b:v','400k','-threads','1','-c:a','libopus','-b:a','96k']],
 ['12-FLV.flv',['-i',base,'-c:v','flv','-q:v','5','-threads','1','-c:a','libmp3lame','-ar','44100','-b:a','128k']]
];for(const [name,args]of definitions){const path=join(out,name);if(!existsSync(path)){console.log('Preparing '+name);ff([...args,path]);}}
const names=['01-MP4-H264-AAC.mp4',...definitions.map(d=>d[0])],records=names.map(name=>{const path=join(out,name),info=JSON.parse(execFileSync(join(tools,'ffprobe.exe'),['-v','error','-show_streams','-show_format','-of','json',path],{encoding:'utf8',windowsHide:true}));return {name,path,sha256:createHash('sha256').update(readFileSync(path)).digest('hex'),duration:Number(info.format.duration),container:info.format.format_name,streams:info.streams.map(s=>({codec:s.codec_name,type:s.codec_type,width:s.width,height:s.height}))};});
writeFileSync(join(root,'matrix.json'),JSON.stringify({officialSources:['https://samples.ffmpeg.org/V-codecs/WMV9/halo2_wmp9_WMV3+audio0x162.wmv','https://samples.ffmpeg.org/V-codecs/WVC1/FlightSimX_720p60_51_15Mbps.wmv'],records},null,2));console.log(JSON.stringify(records.map(r=>({name:r.name,streams:r.streams.map(s=>s.codec),duration:r.duration}))));
