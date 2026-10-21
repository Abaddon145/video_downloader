export function installLearningLayoutFixture(){
  window.calls=[];window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
  const settings={autoplayNext:false,completionThreshold:.9,defaultPlaybackRate:1,restorePosition:true,autoSubtitles:false,playbackPreference:'auto',playbackCacheGb:2,playbackCacheDays:30};
  const series=Array.from({length:2},(_,n)=>({id:`s${n}`,categoryId:'c0',title:`布局验收系列 ${n+1}`,description:'布局回归',coverPath:null,sourceType:'local',sourceUrl:null,sourcePlaylistId:null,author:null,totalItems:2,completedItems:n,totalDurationMs:900000,favorite:false,sortOrder:n,createdAt:1,updatedAt:1,availableItems:2,aiCompletedItems:1,aiActiveItems:0,aiFailedItems:0,lastPlayedAt:1}));
  const item={id:'i0',seriesId:'s0',title:'布局验收课程',filePath:'C:/mock.mp4',sourceUrl:null,sourceVideoId:null,sourcePlaylistIndex:null,durationMs:900000,sortOrder:0,thumbnailPath:null,thumbnailSource:null,sourceThumbnailUrl:null,thumbnailKey:null,downloadTaskId:null,available:true,downloadState:'local',progress:{itemId:'i0',positionMs:30000,durationMs:900000,completed:false,playbackRate:1,lastPlayedAt:1}};
  let callbacks=new Map(),next=0;
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:fn=>{callbacks.set(++next,fn);return next;},unregisterCallback:id=>callbacks.delete(id),convertFileSrc:p=>p,invoke:async(command,args={})=>{
   window.calls.push({command,args});
   if(command==='plugin:event|listen')return ++next;
   if(command==='workspace_get')return JSON.parse(localStorage.getItem('qa-workspace')||'null');
   if(command==='workspace_save'){localStorage.setItem('qa-workspace',JSON.stringify(args.settings));return args.settings;}
   if(command==='get_snapshot')return {settings:{downloadDir:'C:/downloads',concurrency:2,cookieMode:'none',browser:'edge',browserProfile:'',hasCookieFile:false,proxyEnabled:false,proxyUrl:'',autoCheckCoreUpdate:true,lastCoreUpdateCheck:null,cookieSummary:null,downloadPresets:[]},tasks:[],engine:{ready:true,version:'test',ffmpegVersion:'test',denoVersion:'test',error:null},notice:null,updating:false,previewing:false,coreUpdate:null};
   if(command==='get_media_settings')return null;
   if(command==='learning_thumbnail_tasks')return [];
   if(command==='get_media_snapshot')return {tasks:[],ready:true,error:null};
   if(command==='ai_list_analyses')return {items:[],total:0,page:1};
   if(command==='learning_home')return {settings,categories:[{id:'c0',name:'设计',description:null,icon:'BookOpen',color:null,sortOrder:0}],series:args.sort==='title'?[...series].reverse():series,total:2,page:1,continueLearning:[{series:series[0],item}]};
   if(command==='learning_series')return {series:series.find(s=>s.id===args.id),items:[item],total:1,page:1};
   if(command==='learning_item')return item;
   if(command==='learning_prepare_player')return {item,series:series[0],settings,previous:null,next:null,path:item.filePath,subtitles:[],subtitleWarning:null,plan:{mode:'native',format:'mp4',reason:'模拟原生播放',cached:false},technical:'模拟，不运行mpv'};
   if(command==='learning_native_start'||command==='learning_native_control')return {session:args.session,itemId:'i0',ready:true,position:30,duration:900,rate:1,paused:true,volume:.8,muted:false,ended:false,subtitles:[],subtitleId:null,error:null};
   if(command==='learning_native_save_progress')return item.progress;
   if(command==='learning_notes')return {notes:[],total:0,page:1};
   if(command==='ai_item_statuses')return {};
   if(command==='ai_for_item')return null;
   if(command==='learning_change')return args.change.series?.id??'s0';
   return null;
  }};
}
