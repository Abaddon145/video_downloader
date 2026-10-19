export function installKnowledgeFixture(){
  window.isTauri=true;window.calls=[];let next=1;const callbacks=new Map(),listeners=new Map();
  const settings={model:'deepseek-flash',defaultMode:'standard',whisperModel:'base',allowTranscript:true,allowImages:true,cleanTemporaryFiles:true,autoGenerateNotes:true,autoAnalyzeDownloads:false,chunkConcurrency:1};
  const summary={contentType:'tutorial',oneSentenceSummary:'通过 GPU 碰撞与距离场，让 Niagara 粒子正确响应场景。',overview:'本课程从 Niagara System 的基础设置开始，演示 GPU Compute Sim、碰撞参数及距离场调试流程。',chapters:[{startMs:0,endMs:222000,title:'课程介绍与基础设置',summary:'认识粒子系统和本节课程目标。',keywords:[],keyframeIds:[]},{startMs:751000,endMs:810000,title:'GPU Collision',summary:'设置 GPU 模拟目标，并启用碰撞模块。',keywords:[],keyframeIds:[]}],keyPoints:[{timeMs:751000,title:'GPU 模拟需要匹配碰撞方式',content:'先检查模拟目标，再使用场景深度或距离场进行碰撞。',importance:4,sourceSegmentIds:['s0']}],tutorialSteps:[{order:1,timeMs:751000,title:'切换模拟目标',description:'将 Sim Target 设置为 GPU Compute Sim。',notes:[]}],terms:[{name:'Distance Field',definition:'描述空间距离的场数据，可用于粒子碰撞。',timeMs:810000}],actionItems:['在自己的场景中验证一次碰撞效果'],tags:['UE5','Niagara','GPU-Collision']};
  const base={itemId:null,sourceFingerprint:'fake',analysisMode:'standard',model:'deepseek-flash',promptVersion:'v0.5.0-1',language:'zh',durationMs:2301000,progress:1,stage:'视频分析完成',summary,markdownPath:null,markdownVaultId:null,error:null,createdAt:Date.now(),updatedAt:Date.now(),completedAt:Date.now(),options:{localOnly:false,transcribeAudio:true,allowTranscript:true,allowImages:true,whisperModel:'base',autoGenerateNote:true},transcriptDone:true,keyframesDone:true};
  let records=[{...base,id:'a',sourcePath:'D:\\课程\\UE5 Niagara Beginner Tutorial.mp4',status:'completed'},{...base,id:'b',sourcePath:'D:\\课程\\Distance Fields.mp4',status:'failed',summary:null,stage:'分析失败，可重试',error:'DeepSeek 请求失败：HTTP 503'},{...base,id:'c',sourcePath:'D:\\课程\\材质基础.wmv',status:'interrupted',summary:null,stage:'分析被中断，可继续分析'}];
  window.emitAI=(payload,event='ai-analysis-updated')=>{for(const [id,value] of listeners){if(value.event===event)callbacks.get(value.handler)?.({event,id,payload});}};
  window.records=records;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback(callback){const id=next++;callbacks.set(id,callback);return id;},unregisterCallback(id){callbacks.delete(id);},convertFileSrc:path=>path,invoke:async(command,args={})=>{
   window.calls.push({command,args});
   if(command==='plugin:event|listen'){const id=next++;listeners.set(id,args);return id;}if(command==='plugin:event|unlisten'){listeners.delete(args.eventId);return;}
   if(command==='plugin:dialog|open')return 'D:\\课程\\新课程 #Niagara.mp4';
   if(command==='get_snapshot')return {settings:{downloadDir:'D:/下载',concurrency:2,cookieMode:'none',browser:'edge',browserProfile:'',hasCookieFile:false,proxyEnabled:false,proxyUrl:'',autoCheckCoreUpdate:true,lastCoreUpdateCheck:null,cookieSummary:null,downloadPresets:[]},tasks:[],engine:{ready:true,version:'test',ffmpegVersion:'test',denoVersion:'test',error:null},notice:null,updating:false,previewing:false,coreUpdate:null};
   if(command==='get_media_settings')return null;
   window.kb??={configured:false,rev:1,notes:{'映流/课程总结.md':'# GPU Collision\n\n本节介绍 Niagara。\n\n[[映流/目标#重点|关联知识]]\n\n![[Attachments/画面.png]]\n\n<script>window.MARKDOWN_EXECUTED=true</script>','映流/第三.md':'# 第三篇笔记\n\n[播放课程](yingliu://video/lesson?t=751000)','映流/目标.md':'# 目标\n## 重点\n知识归用户所有。'}};
   const kb=window.kb,kid='vault-test';const settings={vaultPath:'D:/Vault',vaultId:kid,rootFolder:'映流',attachmentsFolder:'Attachments',obsidian:kb.obsidian??null};
   const doc=path=>({path,text:kb.notes[path],hash:`mock:${kb.notes[path]}`,mtime:1});
   if(command==='knowledge_get_settings')return kb.configured?settings:{...settings,vaultPath:null,vaultId:null};
   if(command==='knowledge_default_path')return 'D:/Documents/映流知识库';
   if(command==='knowledge_set_vault'){kb.configured=true;return settings;}
   if(command==='knowledge_tree'){const all=args.parent===''?(kb.many?Array.from({length:110},(_,i)=>({path:`root-${i}.md`,name:`root-${i}.md`,directory:false})):[{path:'映流',name:'映流',directory:true}]):Object.keys(kb.notes).map(path=>({path,name:path.split('/').at(-1),directory:false}));return {vaultId:kid,data:{items:all.slice((args.page-1)*50,args.page*50),total:all.length,page:args.page,skipped:0}};}
   if(command==='knowledge_read'){if(window.kb.readDelay)await new Promise(r=>setTimeout(r,window.kb.readDelay));if(!(args.path in kb.notes))throw '笔记不存在';return {vaultId:kid,data:doc(args.path)};}
   if(command==='knowledge_write'){if(args.expectedHash!==doc(args.path).hash)return {vaultId:kid,outcome:{status:'conflict',document:doc(args.path)},indexWarning:null};kb.notes[args.path]=args.text;kb.rev++;return {vaultId:kid,outcome:{status:'saved',document:doc(args.path)},indexWarning:null};}
   if(command==='knowledge_create'){if(args.path in kb.notes)throw '文件已存在';if(!args.directory)kb.notes[args.path]=args.text;return {vaultId:kid,path:args.path,indexWarning:null};}
   if(command==='knowledge_rename'||command==='knowledge_move'){const old=args.path??args.from;kb.notes[args.to]=kb.notes[old];delete kb.notes[old];return {vaultId:kid,path:args.to,indexWarning:null};}
   if(command==='knowledge_delete'){delete kb.notes[args.path];return {vaultId:kid,path:args.path,indexWarning:null};}
   if(command==='knowledge_search')return {vaultId:kid,data:{items:Object.keys(kb.notes).filter(path=>!args.query||kb.notes[path].includes(args.query)).map(path=>({kind:'note',path,title:path.split('/').at(-1),excerpt:kb.notes[path].slice(0,80),itemId:null,analysisId:null,startMs:null,endMs:null})),total:Object.keys(kb.notes).filter(path=>!args.query||kb.notes[path].includes(args.query)).length,page:1}};
   if(command==='knowledge_reindex')return {vaultId:kid,data:{complete:true,indexed:2,skipped:0,warnings:[]}};
   if(command==='knowledge_tags')return {vaultId:kid,data:['UE5','Niagara']};
   if(command==='knowledge_backlinks')return {vaultId:kid,data:{items:[{path:'映流/课程总结.md',title:'GPU Collision'}],total:1,page:1}};
   if(command==='knowledge_resolve'){if(window.kb.resolveDelay)await new Promise(r=>setTimeout(r,window.kb.resolveDelay));window.kb.resolved=true;return {vaultId:kid,data:[{path:'映流/目标.md',title:'目标'}]};}
   if(command==='knowledge_asset')return {vaultId:kid,data:{mime:'image/png',base64:'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+c+o8AAAAASUVORK5CYII='}};
   if(command==='knowledge_analysis_note')return window.generatedNote??null;
   if(command==='knowledge_obsidian_connection'){const b=kb.obsidian??null,protocol=kb.protocolAvailable!==false;return {vaultId:kid,vaultPath:settings.vaultPath,binding:b,protocolAvailable:protocol,lastRequest:null,state:!protocol?'protocolUnavailable':!b?'unconfigured':!b.registrationConfirmed?'registrationPending':b.confirmedByUser?'userConfirmed':'verificationPending'};}
   if(command==='knowledge_configure_obsidian'){kb.obsidian={yingliuVaultId:kid,kind:args.kind,obsidianVaultIdentifier:args.identifier,registrationConfirmed:args.registrationConfirmed,confirmedByUser:false};return {...settings,obsidian:kb.obsidian};}
   if(command==='knowledge_open_in_obsidian'){if(!kb.obsidian?.registrationConfirmed)throw 'Vault 尚未登记';kb.obsidianAttempts??=[];kb.obsidianAttempts.push(args.path);return {vaultId:kid,path:args.path,status:kb.obsidianFail?'launchFailed':'launchAccepted',message:kb.obsidianFail?'打开请求失败：模拟协议异常':'已向 Obsidian 发送打开请求；请确认目标笔记是否已打开'};}
   if(command==='knowledge_confirm_obsidian'){if(JSON.stringify(args.expected)!==JSON.stringify(kb.obsidian))throw '配置已变化';kb.obsidian.confirmedByUser=true;return {...settings,obsidian:kb.obsidian};}
   if(command==='knowledge_obsidian_manager'||command==='knowledge_open_vault_folder')return;

   const lesson={id:'lesson',seriesId:'series',title:'GPU 碰撞课程',filePath:records.find(a=>a.id==='a').sourcePath,sourceUrl:null,durationMs:2301000,sortOrder:0,available:true,downloadState:'completed',thumbnailPath:null,thumbnailSource:null,sourceThumbnailUrl:null,thumbnailKey:null,downloadTaskId:null,progress:{itemId:'lesson',positionMs:0,durationMs:2301000,completed:false,playbackRate:1,lastPlayedAt:null}};
   const series={id:'series',title:'UE5 系列课程',categoryId:null,description:'本地课程',coverPath:null,sourceUrl:null,totalItems:1,completedItems:0,totalDurationMs:2301000,favorite:false,sortOrder:0,createdAt:1,updatedAt:1};
   const learningSettings={autoplayNext:false,completionThreshold:.95,defaultPlaybackRate:1,restorePosition:true,autoSubtitles:true,playbackPreference:'auto',playbackCacheGb:10,playbackCacheDays:30};
   window.courseItem=lesson;
   window.nativeState??={session:'',itemId:'lesson',ready:true,position:0,duration:2301,rate:1,paused:true,volume:.8,muted:false,ended:false,subtitles:[],subtitleId:null,error:null};
   if(command==='learning_home')return {categories:[],series:[series],total:1,page:1,continueLearning:[],settings:learningSettings};
   if(command==='learning_series')return {series,items:[lesson],page:1,total:1};
   if(command==='learning_prepare_player')return {item:lesson,series,settings:learningSettings,previous:null,next:null,path:lesson.filePath,subtitles:[],subtitleWarning:null,plan:{mode:'native',format:'mp4',reason:'直接播放原文件',cached:false},technical:'模拟播放元信息'};
   if(command==='learning_native_start'){window.nativeState.session=args.session;return window.nativeState;}
   if(command==='learning_native_control'){if(args.action.type==='seek')window.nativeState.position=args.action.position;if(args.action.type==='pause')window.nativeState.paused=args.action.paused;return {...window.nativeState};}
   if(command==='learning_native_save_progress')return {...lesson.progress,positionMs:window.nativeState.position*1000};
   if(command==='learning_item')return lesson;
   if(command==='learning_notes')return {notes:window.savedNotes??[],total:window.savedNotes?.length??0,page:1};
   if(command==='learning_save_note'){const n={...args.input,id:'note-1',createdAt:1,updatedAt:1};window.savedNotes=[n];return n;}
   if(command==='ai_item_statuses')return window.courseReady?{lesson:{id:'a',status:'completed',progress:1,hasSummary:true,createdAt:1,updatedAt:1}}:{};
   if(command==='ai_item_analyses')return {items:records.filter(a=>a.id==='a').map(a=>({...a,itemId:args.itemId})),total:1,page:1};
   if(command==='ai_for_item')return window.courseReady&&records.find(a=>a.id==='a')?{...records.find(a=>a.id==='a'),itemId:'lesson'}:null;

   if(command==='ai_get_settings')return {settings,hasApiKey:true};
   if(command==='ai_list_analyses'){const items=records.filter(r=>!args.status||r.status===args.status||args.status==='active'&&r.status==='queued');return {items,total:items.length,page:1};}
   if(command==='ai_library_videos'){const all=Array.from({length:55},(_,i)=>({itemId:`lesson-${i}`,title:`第${i}课 Niagara`,seriesTitle:'UE5 系列',path:i===1?null:`D:\\课程\\第${i}课.mp4`,durationMs:600000,available:i!==1})).filter(v=>!args.query||`${v.title} ${v.seriesTitle}`.includes(args.query));return {items:all.slice((args.page-1)*50,args.page*50),total:all.length,page:args.page};}
   if(command==='ai_analysis')return records.find(r=>r.id===args.id);
   if(command==='ai_whisper_models')return [{id:'base',label:'Whisper Base',bytes:147951465,installed:false},{id:'small',label:'Whisper Small',bytes:487601967,installed:false}];
   if(command==='ai_model_download')return null;
   if(command==='ai_download_whisper_model')return {id:'model-1',modelId:args.modelId,status:'downloading',received:100,total:147951465,error:null};
   if(command==='ai_cancel_model_download')return;
   if(command==='ai_course_transcript'){const at=args.currentMs,pg=at==null?args.page:Math.floor(Math.max(0,at)/500000)+1,page=Math.min(2,Math.max(1,pg)),start=(page-1)*50;return {source:window.existingSubtitle?'subtitle':'whisper',analysisId:window.existingSubtitle?null:'a',warning:null,nextStartMs:page===1?500000:null,total:75,page,items:Array.from({length:Math.min(50,75-start)},(_,i)=>({id:`s${start+i}`,analysisId:'a',segmentIndex:start+i,startMs:(start+i)*10000,endMs:(start+i)*10000+9000,text:`第${start+i+1}段：这里介绍 Niagara 粒子的碰撞设置。`}))};}
   if(command==='ai_transcript'){const start=(args.page-1)*50;return {total:75,page:args.page,items:Array.from({length:Math.min(50,75-start)},(_,i)=>({id:`s${start+i}`,analysisId:args.id,segmentIndex:start+i,startMs:(start+i)*10000,endMs:(start+i)*10000+9000,text:`第${start+i+1}段：这里介绍 Niagara 粒子的碰撞设置。`}))};}
   if(command==='ai_ask_video')return {id:'question-1',analysisId:args.id,answer:'课程在12:31开始介绍GPU碰撞。',references:[{startMs:751000,endMs:810000,label:'GPU Collision'}]};
   if(command==='ai_usage')return {analyses:3,questions:2,models:[{model:'deepseek-flash',inputTokens:3820000,outputTokens:410000,cachedTokens:100000,unknownRequests:0}]};
   if(command==='ai_cache_info'||command==='ai_clear_cache')return {bytes:180000000,clearableBytes:30000000};
   if(command==='ai_start_analysis'||command==='ai_start_transcription'){const r={...base,id:`new-${records.length}`,title:args.request?.itemId==='lesson-37'?'第37课 Niagara':undefined,itemId:args.itemId??args.request?.itemId??null,sourcePath:args.path??args.request.path,status:'queued',summary:null,progress:0,stage:'等待分析',options:{...base.options,localOnly:command==='ai_start_transcription'}};records.unshift(r);return r;}
   if(command==='ai_retry_analysis'){const r=records.find(r=>r.id===args.id);r.status='queued';return {...r,updatedAt:Date.now()+3000};}
   if(command==='ai_cancel_analysis'){records.find(r=>r.id===args.id).status='cancelled';return;}
   if(command==='ai_delete_analysis'){records=records.filter(r=>r.id!==args.id);return null;}
   return 1;
  }};
 }
