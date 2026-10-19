export function installNoteToolsFixture(){
 const original=window.__TAURI_INTERNALS__.invoke;let metadata={},versions={},sequence=0,settings=null;
 window.qaNoteAi=[];window.qaNoteAttachments=[];
 const view=data=>({vaultId:'vault-test',data:structuredClone(data)});
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==='knowledge_write'){const text=window.kb.notes[args.path];const r=await original(command,args);if(r.outcome.status==='saved'&&text!==r.outcome.document.text){versions[args.path]??=[];versions[args.path].unshift({id:`version-${++sequence}`,path:args.path,hash:`mock:${text}`,text,bytes:new TextEncoder().encode(text).length,createdAt:Date.now()+sequence});}return r;}
  if(command==='ai_get_settings'){const r=await original(command,args);settings??={...r.settings,allowNoteText:false};return {...r,settings};}
  if(command==='ai_save_settings'){settings={...args.settings};return {hasApiKey:true,settings};}
  if(command==='knowledge_note_metadata')return view({path:args.path,favorite:false,template:false,available:true,...metadata[args.path]});
  if(command==='knowledge_set_note_metadata'){metadata[args.path]={path:args.path,favorite:args.favorite,template:args.template,available:true};return view(metadata[args.path]);}
  if(command==='knowledge_note_collection'){const items=Object.values(metadata).filter(m=>args.kind==='favorites'?m.favorite:m.template);return view({items,total:items.length,page:1});}
  if(command==='knowledge_note_versions'){const items=versions[args.path]??[];return view({items,total:items.length,page:1});}
  if(command==='knowledge_note_version')return view(versions[args.path].find(v=>v.id===args.versionId));
  if(command==='knowledge_attachments')return view({items:window.qaNoteAttachments,total:window.qaNoteAttachments.length,page:1});
  if(command==='knowledge_import_attachment'){const a={path:'Attachments/mock-attachment.jpg',name:'中文 画面.jpg',hash:'mock-image',bytes:128,importedAt:1,state:'ready'};window.qaNoteAttachments.push(a);return view(a);}
  if(command==='knowledge_export_attachment')return true;
  if(command==='knowledge_delete_attachment'){window.qaNoteAttachments=window.qaNoteAttachments.filter(a=>a.path!==args.path);return {vaultId:'vault-test',path:args.path,indexWarning:null};}
  if(command==='ai_organize_note'){window.qaNoteAi.push(structuredClone(args.request));const inputHash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(args.request.text)))].map(b=>b.toString(16).padStart(2,'0')).join('');return {requestId:args.request.requestId,vaultId:args.request.vaultId,path:args.request.path,sourceHash:args.request.expectedHash,inputHash,markdown:'# AI整理预览\n\n只整理已授权的笔记。'};}
  if(command==='ai_cancel_note_organize')return;
  return original(command,args);
 };
}
