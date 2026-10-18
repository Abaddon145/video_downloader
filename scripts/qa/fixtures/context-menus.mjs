export function installContextMenuFixture(){
 window.isTauri=true;
 const original=window.__TAURI_INTERNALS__.invoke;
 const baseTask=(status,n)=>({id:`d${n}`,status,request:{url:`https://example.com/${n}`,title:`下载菜单 ${n}`,kind:'video',videoMode:'compatible',maxHeight:0,subtitleLanguages:[],thumbnail:null},progress:{percent:20,downloaded:100,total:500,speed:10,eta:40},phase:'本地模拟',outputDir:'C:/downloads',files:status==='completed'?[`C:/downloads/video${n}.mp4`]:[],error:null,logs:[],createdAt:n,finishedAt:status==='completed'?n:null,queueOrder:n});
 const downloads=['downloading','downloading','paused','completed'].map(baseTask);
 const media=Array.from({length:2},(_,n)=>({id:`m${n}`,type:'transcode',inputPath:`C:/media/input${n}.mp4`,outputPath:`C:/media/output${n}.mp4`,request:{operation:'transcode',outputFormat:'mp4',inputPath:`C:/media/input${n}.mp4`,outputDir:'C:/media'},status:'completed',phase:'完成',progress:100,speed:null,processedTime:10,totalDuration:10,eta:0,error:null,logs:[],createdAt:n,finishedAt:n}));
 const notes={'映流/当前.md':'# 当前笔记\n保留内容','映流/右键目标.md':'# 右键目标\n正确对象'};
 const kid='menu-vault';
 const document=path=>{if(!(path in notes))throw '笔记不存在';return {path,text:notes[path],hash:`hash:${notes[path]}`,mtime:1};};
 window.qaNotes=notes;
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  window.calls.push({command,args});
  if(command==='plugin:app|version')return '0.5.0';
  if(command==='get_snapshot')return {...await original(command,args),tasks:downloads};
  if(command==='batch_task_action')return {succeeded:args.ids,skipped:[],failed:[]};
  if(command==='get_media_snapshot')return {tasks:media,ready:true,error:null};
  if(command==='discover_media')return args.paths;
  if(command==='remove_media_records')return;
  if(command==='knowledge_get_settings')return {vaultPath:'C:/menu-vault',vaultId:kid,rootFolder:'映流',attachmentsFolder:'Attachments'};
  if(command==='knowledge_tree')return {vaultId:kid,data:{items:args.parent===''?[{path:'映流',name:'映流',directory:true}]:Object.keys(notes).map(path=>({path,name:path.split('/').at(-1),directory:false})),total:args.parent===''?1:Object.keys(notes).length,page:1,skipped:0}};
  if(command==='knowledge_read')return {vaultId:kid,data:document(args.path)};
  if(command==='knowledge_reindex')return {vaultId:kid,data:{indexed:2,skipped:0,complete:true,warnings:[]}};
  if(command==='knowledge_tags')return {vaultId:kid,data:[]};
  if(command==='knowledge_backlinks')return {vaultId:kid,data:{items:[],total:0,page:1}};
  if(command==='knowledge_move'||command==='knowledge_rename'){const source=args.from??args.path;notes[args.to]=notes[source];delete notes[source];return {vaultId:kid,path:args.to,indexWarning:null};}
  if(command==='knowledge_delete'){if(args.expectedHash!==document(args.path).hash)throw '文件已变化';delete notes[args.path];return {vaultId:kid,path:args.path,indexWarning:null};}
  if(command==='knowledge_create'){if(args.path in notes)throw '文件已存在';notes[args.path]=args.text;return {vaultId:kid,path:args.path,indexWarning:null};}
  if(command==='knowledge_obsidian_connection')return {vaultId:kid,vaultPath:'C:/menu-vault',binding:null,state:'unconfigured',protocolAvailable:false,lastRequest:null};
  return original(command,args);
 };
}
