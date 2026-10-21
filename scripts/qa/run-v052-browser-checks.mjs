import {spawn} from 'node:child_process';
import {resolve} from 'node:path';
const port=Number(process.env.QA_PORT??5290),base=`http://127.0.0.1:${port}`;
const server=spawn(process.execPath,[resolve('node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port',String(port),'--strictPort'],{windowsHide:true,stdio:'ignore'});
const defaults=[['commands','commands-ui'],['live-preview','editor-ui'],['player-workspace','player-ui'],['tasks','tasks-ui'],['subtitle-workflow','subtitle-ui'],['recovery','recovery-ui'],['performance','performance']];
const selected=process.argv.slice(2),suite=selected.length?defaults.filter(([name])=>selected.includes(name)):defaults;
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
try{
 let ready=false;for(let i=0;i<100;i++){if(server.exitCode!==null)throw Error('Local QA server did not start');try{ready=(await fetch(base,{signal:AbortSignal.timeout(500)})).ok;}catch{}if(ready)break;await sleep(100);}if(!ready)throw Error('Local QA server did not become ready');
 for(const [name,folder] of suite){await new Promise((resolve,reject)=>{const child=spawn(process.execPath,[`scripts/qa/verify-v052-${name}.mjs`],{windowsHide:true,stdio:'inherit',env:{...process.env,QA_BASE_URL:base,QA_OUTPUT_DIR:`docs/evidence/v0.5.2/phase7/${folder}`}});child.once('error',reject);child.once('exit',code=>code===0?resolve():reject(Error(`${name} exited ${code}`)));});}
 console.log(`PASS ${suite.length} isolated mocked browser suites`);
}finally{server.kill();}
