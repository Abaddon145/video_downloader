// Legacy workflow tests deliberately use one known purpose; full template coverage lives in verify-ai-templates-ui.mjs.
export function installGeneralTemplateFixture(){
 const original=window.__TAURI_INTERNALS__.invoke;
 window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
  if(command==='ai_list_templates')return [{id:'general',version:1,name:'通用总结',description:'既有流程的模拟通用模板',focus:'提取课程重点',sections:[{id:'summary',title:'内容总结'}],detail:'balanced',extractParameters:false,extractTerms:true,exercises:false,timeReferences:true,markdownTemplate:'',builtin:true,favorite:false}];
  return original(command,args);
 };
}
