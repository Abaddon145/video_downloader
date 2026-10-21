import {spawn} from 'node:child_process';
import net from 'node:net';
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const out=resolve(process.env.QA_OUTPUT_DIR??'docs/evidence/v0.5.2/phase4/mpv');mkdirSync(out,{recursive:true});
const pipe=`\\\\.\\pipe\\yingliu-v052-bindings-${process.pid}`,pending=new Map(),events=[],checks=[];let serial=0,buffer='',socket;
const mpv=spawn(resolve('src-tauri/resources/tools/mpv.exe'),['--no-config','--idle=yes','--vo=null','--ao=null','--input-terminal=no','--input-default-bindings=no',`--input-ipc-server=${pipe}`],{windowsHide:true,stdio:'ignore'});
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const command=(...command)=>new Promise((resolve,reject)=>{const request_id=++serial;const timer=setTimeout(()=>{pending.delete(request_id);reject(Error('mpv command timeout'));},3000);pending.set(request_id,result=>{clearTimeout(timer);result.error==='success'?resolve(result.data):reject(Error(result.error));});socket.write(JSON.stringify({command,request_id})+'\n');});
try{
 for(let i=0;i<60;i++){try{socket=await new Promise((resolve,reject)=>{const client=net.connect(pipe);client.once('connect',()=>resolve(client));client.once('error',reject);});break;}catch{await sleep(100);}}
 assert.ok(socket,'bundled mpv starts');socket.on('data',data=>{buffer+=data;for(let i;(i=buffer.indexOf('\n'))>=0;){const value=JSON.parse(buffer.slice(0,i));buffer=buffer.slice(i+1);if(value.request_id){pending.get(value.request_id)?.(value);pending.delete(value.request_id);}if(value.event==='client-message')events.push(value.args);}});
 for(const key of ['SPACE','j','M','Ctrl+k','Ctrl+!','+','Ctrl++','Shift+LEFT']){
  const id=`player.test-${checks.length}`;await command('keybind',key,`script-message yingliu-command:${id}`);const before=events.length;await command('keypress',key);for(let i=0;i<30&&events.length===before;i++)await sleep(20);assert.deepEqual(events.at(-1),[`yingliu-command:${id}`]);await command('keybind',key,'ignore');const count=events.length;await command('keypress',key);await sleep(30);assert.equal(events.length,count,'cleared binding cannot fire');checks.push({key,passed:true});
 }
 console.log(`PASS ${checks.length} real bundled mpv IPC key binding checks`);
}finally{writeFileSync(resolve(out,'checks.json'),JSON.stringify({realBundledMpv:true,simulatedKeypressOverIpc:true,physicalKeyboard:false,checks},null,2));socket?.destroy();mpv.kill();}
