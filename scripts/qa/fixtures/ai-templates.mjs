export function installAiTemplateFixture(){
 window.isTauri=true;
 const original=window.__TAURI_INTERNALS__.invoke;
 const definitions=[['general','通用总结'],['course','课程学习'],['software','软件操作教程'],['ue5-cg','UE5 / CG制作'],['film','影视镜头分析'],['programming','编程开发'],['language','语言学习'],['meeting','会议访谈'],['troubleshooting','故障排查'],['custom-starter','自定义模板']].map(([id,name])=>({id,version:1,name,description:`${name}的本地模拟用途说明`,focus:`围绕${name}提取有来源的知识。`,sections:[{id:'goals',title:'学习目标'},{id:'practice',title:'实践练习'}],detail:'balanced',extractParameters:false,extractTerms:true,exercises:true,timeReferences:true,markdownTemplate:'',builtin:true,favorite:false}));
 const versions=new Map(definitions.map(t=>[t.id,[structuredClone(t)]]));
 let nextTemplate=0,nextAnalysis=0;
 const analyses=[];
 window.qaTemplates=definitions;window.qaAnalyses=analyses;
 const settings={model:'deepseek-flash',defaultMode:'standard',whisperModel:'base',allowTranscript:true,allowImages:true,cleanTemporaryFiles:true,autoGenerateNotes:true,autoAnalyzeDownloads:false,chunkConcurrency:1};
 const result=template=>({contentType:'tutorial',oneSentenceSummary:'通过类型和测试验证课程中的实现。',overview:'本地模拟模板结果，用于检查展示与历史隔离。',chapters:[{startMs:0,endMs:60000,title:'课程目标',summary:'介绍实现与验证',keywords:[],keyframeIds:[]}],keyPoints:[],tutorialSteps:[],terms:[],actionItems:[],tags:['编程'],sections:template.sections.map(s=>({id:s.id,title:s.title,items:[{title:`${s.title}示例`,content:'根据课程内容整理的测试示例。',timeMs:30000,sourceSegmentIds:['fixture-segment'],keyframeIds:[]}]}))});
 const create=(request,template)=>{const a={id:`qa-analysis-${++nextAnalysis}`,itemId:request.itemId,sourcePath:request.path,sourceFingerprint:'mock-only',status:'completed',analysisMode:request.mode,model:settings.model,promptVersion:'v0.5.0-1',language:'zh',durationMs:900000,progress:1,stage:'完成',summary:result(template),markdownPath:null,markdownVaultId:null,error:null,noteWarning:null,noteTargetVaultId:null,createdAt:Date.now()+nextAnalysis,updatedAt:Date.now()+nextAnalysis,completedAt:Date.now(),options:{localOnly:false,transcribeAudio:request.transcribeAudio,allowTranscript:request.allowTranscript,allowImages:request.allowImages,whisperModel:'base',autoGenerateNote:request.autoGenerateNote},transcriptDone:true,keyframesDone:true,templateId:template.id,templateVersion:template.version,templateSnapshot:{...structuredClone(template),favorite:false},templateWarning:null};analyses.push(a);return structuredClone(a);};
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  window.calls.push({command,args});
  if(command==='plugin:app|version')return '0.5.0';
  if(command==='ai_get_settings')return {settings,hasApiKey:!window.qaNoKey};
  if(command==='ai_whisper_models')return [];
  if(command==='ai_list_templates'){if(window.qaTemplateListFailure)throw '用途模板读取失败（模拟）';return structuredClone(definitions);}
  if(command==='ai_template_versions')return structuredClone(versions.get(args.id)??[]);
  if(command==='ai_copy_template'){const source=versions.get(args.id)?.find(t=>t.version===(args.version??versions.get(args.id).at(-1).version));if(!source)throw '模板不存在';const copied={...structuredClone(source),id:`custom-qa-${++nextTemplate}`,version:1,name:`${source.name} - 副本`,builtin:false,favorite:false};definitions.push(copied);versions.set(copied.id,[structuredClone(copied)]);return structuredClone(copied);}
  if(command==='ai_save_template'){let index=definitions.findIndex(t=>t.id===args.template.id),old=definitions[index];if(old?.builtin)throw '官方模板不能覆盖';if(old&&old.version!==args.expectedVersion)throw '模板已被其他窗口修改';const saved={...structuredClone(args.template),id:old?.id??`custom-qa-${++nextTemplate}`,version:(old?.version??0)+1,builtin:false};if(index<0)definitions.push(saved);else definitions[index]=saved;versions.set(saved.id,[...(versions.get(saved.id)??[]),structuredClone(saved)]);return structuredClone(saved);}
  if(command==='ai_set_template_favorite'){const t=definitions.find(t=>t.id===args.id);if(!t)throw '模板不存在';t.favorite=args.favorite;return structuredClone(t);}
  if(command==='ai_delete_template'){const index=definitions.findIndex(t=>t.id===args.id);if(index<0||definitions[index].builtin)throw '只能删除自定义模板';definitions.splice(index,1);return;}
  if(command==='ai_start_analysis'){const t=definitions.find(t=>t.id===args.request.templateId&&t.version===args.request.templateVersion);if(!t)throw '模板已变化';return create(args.request,t);}
  if(command==='ai_for_item')return structuredClone(analyses.filter(a=>a.itemId===args.itemId).at(-1)??null);
  if(command==='ai_item_statuses')return Object.fromEntries(analyses.filter(a=>args.ids.includes(a.itemId)).map(a=>[a.itemId,{id:a.id,status:a.status,progress:1,hasSummary:true,createdAt:a.createdAt,updatedAt:a.updatedAt}]));
  if(command==='ai_item_analyses')return {items:structuredClone(analyses.filter(a=>a.itemId===args.itemId).reverse()),total:analyses.length,page:1};
  if(command==='ai_retry_analysis'){const old=analyses.find(a=>a.id===args.id);return create({path:old.sourcePath,itemId:old.itemId,mode:old.analysisMode,...old.options},old.templateSnapshot);}
  return original(command,args);
 };
}
