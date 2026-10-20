import {useRegisterCommands,useCommands} from '../commands/CommandProvider';
import {lazy,Suspense,useEffect,useMemo,useRef,useState,useSyncExternalStore} from 'react';
import {NoteSessions} from './NoteSessions';
import {NoteTools,type NoteSnapshot,type NoteToolRequest} from './NoteTools';
import {toRichText} from './MarkdownRoundTrip';
const RichTextEditor=lazy(()=>import('./RichTextEditor').then(m=>({default:m.RichTextEditor})));
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {open} from '@tauri-apps/plugin-dialog';
import {BookOpen,FolderOpen,Plus,Save,PanelLeft,PanelRight,RefreshCw,ArrowUpRight,FileText} from 'lucide-react';
import type {BacklinkPage,IndexStats,KnowledgeDocument,KnowledgeMutation,KnowledgeSettings,KnowledgeView,KnowledgeWrite,LeaveGuard,NoteCandidate,SearchPage,KnowledgeOpenRequest,TreeEntry} from '../../types/knowledge';
import {LearningDialog} from '../learning/Editors';
import {aiError} from '../ai/lib';
import {FileTree} from './FileTree';
import {MarkdownEditor} from './MarkdownEditor';
import type {ReactCodeMirrorRef} from '@uiw/react-codemirror';
import {MarkdownPreview} from './MarkdownPreview';
import {ObsidianConnection} from './ObsidianConnection';
import {MAX_OUTLINE_CHARACTERS,headingSlug,headings,relativeNotePath} from './lib';
import {useWorkspace} from '../workspace/WorkspaceProvider';
import {ResizeHandle} from '../../components/ResizeHandle';
import './knowledge.css';

type FileAction='新建笔记'|'新建文件夹'|'重命名笔记'|'移动笔记'|'删除笔记'|'重命名笔记本'|'移动笔记本'|'删除空笔记本'|'复制笔记';
export default function KnowledgePage({notify,registerLeaveGuard,onOpenVideo,openRequest,onOpenConsumed}:{notify:(message:string)=>void;registerLeaveGuard?:(guard:LeaveGuard|null)=>void;onOpenVideo?:(itemId:string,timeMs:number)=>void;openRequest?:KnowledgeOpenRequest|null;onOpenConsumed?:(version:number)=>void}){
 const [settings,setSettings]=useState<KnowledgeSettings|null>(null);
 const [sessions]=useState(()=>new NoteSessions(async(vaultId,path,text,expectedHash)=>{const r=await invoke<KnowledgeWrite>('knowledge_write',{vaultId,path,text,expectedHash});if(alive.current&&r.outcome.status==='saved'){setRevision(v=>v+1);if(r.indexWarning)notify(r.indexWarning);}return r;}));
 const tabs=useSyncExternalStore(sessions.subscribe,sessions.getSnapshot),[activeKey,setActiveKey]=useState<string|null>(null);
 const active=tabs.find(s=>s.key===activeKey),doc=active?.document??null,text=active?.draft??'';
 const setText=(value:string)=>{if(activeKey)sessions.edit(activeKey,value);};
 const [error,setError]=useState(''),[busy,setBusy]=useState(false),[indexing,setIndexing]=useState(false),[revision,setRevision]=useState(0);
 const [mode,setMode]=useState<'编辑'|'富文本'|'预览'|'分屏'>('编辑');
 const [toolRequest,setToolRequest]=useState<NoteToolRequest|null>(null);
 const rich=useMemo(()=>mode==='富文本'?toRichText(text):null,[mode,text]);
 const {workspace,setLayout}=useWorkspace(),layout=workspace.currentLayout.knowledge,left=!layout.treeCollapsed,right=!layout.infoCollapsed;
 const changeLayout=(patch:Partial<typeof layout>)=>setLayout(old=>({...old,knowledge:{...old.knowledge,...patch}}));
 const setLeft=(show:boolean)=>changeLayout({treeCollapsed:!show}),setRight=(show:boolean)=>changeLayout({infoCollapsed:!show});
 const [pending,setPending]=useState<(()=>void)|null>(null),[conflict,setConflict]=useState<KnowledgeDocument|null>(null),[externalView,setExternalView]=useState(false);
 const [action,setAction]=useState<FileAction|null>(null),[path,setPath]=useState(''),[actionError,setActionError]=useState('');
 const [actionSource,setActionSource]=useState<{vaultId:string;path:string;directory:boolean;document:KnowledgeDocument|null}|null>(null);
 const [obsidian,setObsidian]=useState<{path:string|null;autoOpen:boolean}|null>(null),[obsidianDirty,setObsidianDirty]=useState<boolean|null>(null);
 const [query,setQuery]=useState(''),[tag,setTag]=useState(''),[searchPage,setSearchPage]=useState(1),[results,setResults]=useState<SearchPage|null>(null);
 const [tags,setTags]=useState<string[]>([]),[backlinks,setBacklinks]=useState<BacklinkPage|null>(null),[backlinkPage,setBacklinkPage]=useState(1),[choices,setChoices]=useState<NoteCandidate[]|null>(null);
 const editor=useRef<ReactCodeMirrorRef|null>(null),alive=useRef(true),operation=useRef(0),locked=useRef(false),scanBusy=useRef<string|null>(null);
 const current=useRef({settings,doc,text,busy});current.current={settings,doc,text,busy};
 const dirty=!!activeKey&&sessions.dirty(activeKey);
 useEffect(()=>{sessions.setVault(settings?.vaultId??null);},[settings?.vaultId,sessions]);
 useEffect(()=>()=>sessions.dispose(),[sessions]);
 useEffect(()=>{setConflict(active?.conflict??null);},[active?.key,active?.conflict]);
 const outline=useMemo(()=>headings(text),[text]);
 const valid=(id:string)=>alive.current&&current.current.settings?.vaultId===id;
 const guard:LeaveGuard=leave=>{if(locked.current){notify('正在处理笔记，请稍候');return;}if(sessions.dirtyKeys().length)setPending(()=>leave);else leave();};
 const guardRef=useRef(guard);guardRef.current=guard;
 useEffect(()=>{let active=true;let chain=Promise.resolve();const report=()=>{const pending=sessions.dirtyKeys().length>0;chain=chain.then(async()=>{await invoke('knowledge_set_pending_edits',{pending});}).catch(e=>{if(active&&alive.current)setError('退出保护状态未能同步，请先保存笔记。'+String(e));});};const stop=sessions.subscribe(report);report();let unlisten:(()=>void)|undefined;
  void listen('knowledge-exit-requested',()=>{guardRef.current(()=>{void invoke('exit_app',{confirmPending:true}).catch(e=>{if(alive.current)setError(String(e));});});}).then(fn=>active?unlisten=fn:fn()).catch(e=>{if(active)setError(String(e));});
  return()=>{active=false;stop();unlisten?.();chain=chain.then(async()=>{await invoke('knowledge_set_pending_edits',{pending:false});}).catch(()=>{});};
 },[sessions]);
 useEffect(()=>{setObsidian(null);setObsidianDirty(null);},[settings?.vaultId,doc?.path]);
 useEffect(()=>{alive.current=true;const ticket=++operation.current;invoke<KnowledgeSettings>('knowledge_get_settings').then(s=>{if(alive.current&&ticket===operation.current)setSettings(s);}).catch(e=>{if(alive.current)setError(aiError(e));});registerLeaveGuard?.(leave=>guardRef.current(leave));return()=>{alive.current=false;++operation.current;registerLeaveGuard?.(null);};},[registerLeaveGuard]);
 function accept(d:KnowledgeDocument){const id=current.current.settings?.vaultId;if(!id)return;const key=sessions.open(id,d);sessions.refresh(key,d);const s=sessions.get(key)!;current.current={...current.current,doc:s.document,text:s.draft};setActiveKey(key);setConflict(s.conflict);setError('');setBacklinkPage(1);}
 async function closeTab(key:string){await sessions.settle();if(!sessions.close(key))return;if(activeKey===key)setActiveKey(sessions.getSnapshot().filter(s=>s.vaultId===settings?.vaultId).at(-1)?.key??null);}
 async function discardAndContinue(){await sessions.settle();for(const key of sessions.dirtyKeys())sessions.discard(key);const next=pending;setPending(null);next?.();}
 async function saveAndContinue(){if(await sessions.flush()){const next=pending;setPending(null);next?.();}else{const failed=sessions.getSnapshot().find(s=>s.conflict||s.state==='error'||s.state==='paused');if(failed){setActiveKey(failed.key);setConflict(failed.conflict);setError(failed.error||'部分笔记未保存，请处理冲突后重试。');}}}

 async function read(path:string,heading?:string){const id=current.current.settings?.vaultId;if(!id)return;const ticket=++operation.current,ownLock=!locked.current;if(ownLock){locked.current=true;setBusy(true);}try{const r=await invoke<KnowledgeView<KnowledgeDocument>>('knowledge_read',{vaultId:id,path});if(valid(id)&&ticket===operation.current&&r.vaultId===id){accept(r.data);if(heading)requestAnimationFrame(()=>document.getElementById(headingSlug(heading))?.scrollIntoView({block:'nearest'}));}}catch(e){if(valid(id)&&ticket===operation.current)setError(aiError(e));}finally{if(ownLock){locked.current=false;if(alive.current)setBusy(false);}}}
 const openedRequest=useRef(0);
 useEffect(()=>{if(!openRequest||!settings||openedRequest.current===openRequest.version)return;const request=openRequest;openedRequest.current=request.version;
  if(settings.vaultId!==request.vaultId){notify(settings.vaultId?'这篇笔记属于其他知识库，请先切换到对应知识库。':'请先选择这篇笔记所属的知识库。');onOpenConsumed?.(request.version);return;}
  guard(()=>{void read(request.path).finally(()=>onOpenConsumed?.(request.version));});
 },[openRequest?.version,settings?.vaultId,onOpenConsumed]);
 const openNote=(path:string,heading?:string)=>{void read(path,heading);};
 function openObsidian(autoOpen=true){if(locked.current)return;const c=current.current;if(c.doc&&c.text!==c.doc.text)setObsidianDirty(autoOpen);else setObsidian({path:c.doc?.path??null,autoOpen});}
 async function save(expected?:string):Promise<boolean>{if(!activeKey)return true;const ok=await sessions.save(activeKey,expected);const s=sessions.get(activeKey);if(s?.conflict){setConflict(s.conflict);setExternalView(false);}if(s?.error)setError(s.error);if(ok)notify('笔记已保存');return ok;}
 async function prepareNote():Promise<NoteSnapshot>{if(!activeKey||!(await sessions.save(activeKey)))throw Error('请先处理笔记的保存错误或冲突。');const s=sessions.get(activeKey);if(!s||current.current.settings?.vaultId!==s.vaultId||current.current.doc?.path!==s.path)throw Error('当前笔记已切换，请重新操作。');return {vaultId:s.vaultId,path:s.path,hash:s.document.hash,text:s.draft,editVersion:s.editVersion};}
 function applyNote(value:string,snapshot:NoteSnapshot){if(!activeKey||settings?.vaultId!==snapshot.vaultId||doc?.path!==snapshot.path)return false;const s=sessions.get(activeKey);if(!s||s.document.hash!==snapshot.hash||s.editVersion!==snapshot.editVersion)return false;sessions.edit(activeKey,value);return true;}


 useEffect(()=>{const closing=(e:BeforeUnloadEvent)=>{if(sessions.dirtyKeys().length){e.preventDefault();e.returnValue='';}};window.addEventListener('beforeunload',closing);return()=>window.removeEventListener('beforeunload',closing);},[sessions]);

 async function refreshFile(){const c=current.current,id=c.settings?.vaultId;if(!id||!c.doc||locked.current)return;try{const r=await invoke<KnowledgeView<KnowledgeDocument>>('knowledge_read',{vaultId:id,path:c.doc.path});if(!valid(id)||r.vaultId!==id||current.current.doc?.path!==c.doc.path||current.current.doc?.hash!==c.doc.hash||locked.current)return;const now=current.current;if(r.data.hash!==now.doc?.hash){if(now.text!==now.doc?.text){sessions.refresh(sessions.key(id,c.doc.path),r.data);setConflict(sessions.get(sessions.key(id,c.doc.path))?.conflict??null);setExternalView(false);}else sessions.refresh(sessions.key(id,c.doc.path),r.data);}}catch(e){if(valid(id)&&current.current.doc?.path===c.doc.path)setError('无法重新读取当前笔记，编辑内容已保留。'+aiError(e));}}
 async function scan(){const id=current.current.settings?.vaultId;if(!id||scanBusy.current===id)return;scanBusy.current=id;setIndexing(true);try{const r=await invoke<KnowledgeView<IndexStats>>('knowledge_reindex',{vaultId:id});if(valid(id)&&r.vaultId===id){setRevision(v=>v+1);if(!r.data.complete)notify('部分文件未能更新索引，已有笔记仍保留。');await refreshFile();}}catch(e){if(valid(id))setError(aiError(e));}finally{if(scanBusy.current===id){scanBusy.current=null;if(valid(id))setIndexing(false);}}}
 const refreshRef=useRef({scan,refreshFile});refreshRef.current={scan,refreshFile};
 useEffect(()=>{const id=settings?.vaultId;if(!id)return;void refreshRef.current.scan();let active=true;const stops:(()=>void)[]=[];let timer:ReturnType<typeof setTimeout>|undefined;
  const focused=()=>{clearTimeout(timer);timer=setTimeout(()=>{void refreshRef.current.scan();},350);};window.addEventListener('focus',focused);
  for(const name of ['knowledge-file-changed','knowledge-index-updated'])void listen<{id:string;vaultId:string}>(name,({payload})=>{if(!active||payload.vaultId!==id||locked.current)return;setRevision(v=>v+1);if(name==='knowledge-file-changed')void refreshRef.current.refreshFile();}).then(stop=>active?stops.push(stop):stop()).catch(e=>{if(active)setError(aiError(e));});
  return()=>{active=false;clearTimeout(timer);window.removeEventListener('focus',focused);stops.forEach(stop=>stop());};
 },[settings?.vaultId]);
 useEffect(()=>{const id=settings?.vaultId;if(!id)return;let active=true;invoke<KnowledgeView<string[]>>('knowledge_tags',{vaultId:id}).then(r=>{if(active&&r.vaultId===id)setTags(r.data);}).catch(e=>{if(active)setError(aiError(e));});return()=>{active=false;};},[settings?.vaultId,revision]);
 useEffect(()=>{const id=settings?.vaultId;if(!id||!doc){setBacklinks(null);return;}let active=true;invoke<KnowledgeView<BacklinkPage>>('knowledge_backlinks',{vaultId:id,path:doc.path,page:backlinkPage}).then(r=>{if(active&&r.vaultId===id)setBacklinks(r.data);}).catch(e=>{if(active)setError(aiError(e));});return()=>{active=false;};},[settings?.vaultId,doc?.path,backlinkPage,revision]);
 useEffect(()=>{const id=settings?.vaultId;if(!id)return;let active=true;const timer=setTimeout(()=>{if(!query.trim()&&!tag){setResults(null);return;}invoke<KnowledgeView<SearchPage>>('knowledge_search',{vaultId:id,query,page:searchPage,tag:tag||null}).then(r=>{if(active&&r.vaultId===id)setResults(r.data);}).catch(e=>{if(active)setError(aiError(e));});},200);return()=>{active=false;clearTimeout(timer);};},[settings?.vaultId,query,tag,searchPage,revision]);
 async function chooseVault(create:boolean){++operation.current;try{const selected=create?await invoke<string>('knowledge_default_path'):await open({directory:true,multiple:false,title:'选择已有知识库'});if(typeof selected!=='string')return;locked.current=true;setBusy(true);const s=await invoke<KnowledgeSettings>('knowledge_set_vault',{path:selected,create});if(!alive.current)return;++operation.current;current.current={settings:s,doc:null,text:'',busy:false};setSettings(s);sessions.setVault(s.vaultId);setActiveKey(null);setConflict(null);setTags([]);setResults(null);setQuery('');setTag('');setError('');setRevision(v=>v+1);}catch(e){setError(aiError(e));}finally{locked.current=false;if(alive.current)setBusy(false);}}
 async function link(target:string,wiki:boolean){const id=current.current.settings?.vaultId,source=current.current.doc?.path;if(!id||!source)return;
  if(target.startsWith('#')){try{jump(headingSlug(decodeURIComponent(target.slice(1))));}catch{notify('链接格式无效');}return;}
  if(wiki){const ticket=operation.current;try{const r=await invoke<KnowledgeView<NoteCandidate[]>>('knowledge_resolve',{vaultId:id,source,target});if(!valid(id)||r.vaultId!==id||ticket!==operation.current||current.current.doc?.path!==source)return;if(r.data.length===1)openNote(r.data[0].path,target.split('#')[1]?.split('|')[0]);else if(r.data.length)setChoices(r.data);else notify('没有找到关联笔记');}catch(e){setError(aiError(e));}}
  else{const p=relativeNotePath(source,target);if(!p){notify('该链接不在知识库内');return;}openNote(/\.md$/i.test(p)?p:p+'.md',target.split('#')[1]);}
 }
 function jump(id:string){if(mode==='预览'){document.getElementById(id)?.scrollIntoView({block:'nearest'});return;}if(mode==='富文本'&&rich?.mode==='rich'){setMode('预览');requestAnimationFrame(()=>document.getElementById(id)?.scrollIntoView({block:'nearest'}));return;}const h=outline.find(h=>h.id===id),view=editor.current?.view;if(h&&view){view.dispatch({selection:{anchor:h.offset},scrollIntoView:true});view.focus();}}
 function prepare(kind:FileAction,target?:TreeEntry){guard(()=>{void (async()=>{
  const id=current.current.settings?.vaultId;if(!id||locked.current)return;const ticket=++operation.current;locked.current=true;setBusy(true);
  try{
   const source=target??(current.current.doc?{path:current.current.doc.path,name:'',directory:false}:null);
   const needsSource=!kind.startsWith('新建');
   const document=needsSource&&source&&!source.directory?(await invoke<KnowledgeView<KnowledgeDocument>>('knowledge_read',{vaultId:id,path:source.path})).data:null;
   if(!valid(id)||ticket!==operation.current)return;
   if(needsSource&&!source)throw '请先选择笔记或笔记本';
   setActionSource(source?{vaultId:id,path:source.path,directory:source.directory,document}:null);
   const parent=target?.directory?target.path:current.current.settings?.rootFolder??'映流';
   setPath(kind==='新建笔记'?`${parent}/新笔记.md`:kind==='新建文件夹'?`${parent}/新文件夹`:kind==='复制笔记'?source!.path.replace(/\.md$/i,' - 副本.md'):source?.path??'');
   setAction(kind);setActionError('');
  }catch(e){if(valid(id))setError(aiError(e));}finally{locked.current=false;if(alive.current)setBusy(false);}
 })();});}
 async function treeAction(entry:TreeEntry,kind:string){
  if(['open-new-tab','favorite','history','ai'].includes(kind)){await read(entry.path);if(kind!=='open-new-tab')setToolRequest({id:Date.now(),path:entry.path,kind:kind as 'favorite'|'history'|'ai'});return;}
  if(kind==='open'){openNote(entry.path);return;}
  if(kind==='new-note'){prepare('新建笔记',entry);return;}
  if(kind==='new-folder'){prepare('新建文件夹',entry);return;}
  if(kind==='rename'){prepare(entry.directory?'重命名笔记本':'重命名笔记',entry);return;}
  if(kind==='move'){prepare(entry.directory?'移动笔记本':'移动笔记',entry);return;}
  if(kind==='delete'){prepare('删除笔记',entry);return;}
  if(kind==='delete-empty'){prepare('删除空笔记本',entry);return;}
  if(kind==='duplicate'){prepare('复制笔记',entry);return;}
  guard(()=>{void (async()=>{
   const id=current.current.settings?.vaultId;if(!id)return;
   if(kind==='obsidian'||kind==='configure-obsidian'){setObsidian({path:entry.path,autoOpen:kind==='obsidian'});return;}
   if(kind==='folder')await invoke('knowledge_open_entry_folder',{vaultId:id,path:entry.path});
   if(kind==='export'){const exported=await invoke<boolean>('knowledge_export_note',{vaultId:id,path:entry.path});if(exported)notify('Markdown 已导出');}
  })().catch(e=>{if(alive.current)setError(aiError(e));});});
 }
 async function mutate(){const id=settings?.vaultId;if(!id||!action||locked.current)return;locked.current=true;setBusy(true);const kind=action,source=actionSource;
  try{if(!(await sessions.flush(id)))throw '请先处理未保存笔记的冲突，再操作文件。';let r:KnowledgeMutation;if(source&&source.vaultId!==id)throw '知识库已切换，请重新选择操作目标';
   if(kind==='删除笔记'){if(!source?.document)throw '请重新选择笔记';r=await invoke('knowledge_delete',{vaultId:id,path:source.path,expectedHash:source.document.hash});}
   else if(kind==='删除空笔记本'){if(!source?.directory)throw '请重新选择笔记本';r=await invoke('knowledge_delete_empty_directory',{vaultId:id,path:source.path});}
   else if(kind==='新建笔记'||kind==='新建文件夹'||kind==='复制笔记')r=await invoke('knowledge_create',{vaultId:id,path,text:kind==='复制笔记'?source?.document?.text??'':kind==='新建笔记'?'# 新笔记\n':'',directory:kind==='新建文件夹'});
   else{if(!source)throw '请先选择笔记或笔记本';sessions.checkMove(id,source.path,path);r=await invoke(kind.startsWith('移动')?'knowledge_move':'knowledge_rename',{vaultId:id,...(kind.startsWith('移动')?{from:source.path}:{path:source.path}),to:path});}
   if(!valid(id)||r.vaultId!==id)return;setAction(null);setRevision(v=>v+1);if(r.indexWarning)notify(r.indexWarning);
   if(kind.startsWith('删除')){if(source){for(const tab of sessions.getSnapshot().filter(s=>s.vaultId===id&&(s.path===source.path||s.path.startsWith(source.path+'/'))))sessions.close(tab.key,true);if(current.current.doc?.path===source.path)setActiveKey(null);}notify(kind==='删除空笔记本'?'空笔记本已删除':'笔记已移至知识库回收目录');}
   else if(kind.startsWith('移动')||kind.startsWith('重命名')){if(source)sessions.move(id,source.path,r.path);const opened=current.current.doc?.path;if(opened&&source&&(opened===source.path||opened.startsWith(source.path+'/')))await read(r.path+opened.slice(source.path.length));notify('位置已更新');}
   else if(kind!=='新建文件夹')await read(r.path);
  }catch(e){if(valid(id))setActionError(aiError(e));}finally{locked.current=false;if(alive.current)setBusy(false);}
 }
 const {openSearch}=useCommands();
 useRegisterCommands([
  {id:'knowledge.new',title:'新建笔记',category:'知识库',scope:'knowledge',shortcut:'Ctrl+Shift+N',keywords:['note','create','笔记'],enabled:!!settings?.vaultId&&!busy,availabilityReason:'请先连接知识库并等待当前操作结束',execute:()=>prepare('新建笔记')},
  {id:'knowledge.search',title:'搜索笔记',category:'知识库',keywords:['find','note'],execute:openSearch},
  {id:'knowledge.save',title:'保存当前笔记',category:'知识库',scope:'knowledge',shortcut:'Ctrl+S',allowInEditable:true,enabled:!!doc&&!busy,availabilityReason:'请先打开笔记',execute:()=>save()},
  {id:'knowledge.mode',title:'切换实时预览与源码',category:'知识库',enabled:!!doc,availabilityReason:'请先打开笔记',execute:()=>setMode(m=>m==='编辑'?'预览':'编辑')},
  {id:'knowledge.recent',title:'打开最近笔记',category:'知识库',enabled:tabs.some(s=>s.vaultId===settings?.vaultId),availabilityReason:'本次尚未打开笔记',execute:()=>{const note=tabs.filter(s=>s.vaultId===settings?.vaultId).at(-1);if(note)openNote(note.path);}},
  ...([['knowledge.template','创建模板笔记','templates'],['knowledge.history','打开版本历史','history']]as const).map(([id,title,kind])=>({id,title,category:'知识库',enabled:!!doc&&!busy,availabilityReason:'请先打开笔记',execute:()=>{if(doc)setToolRequest({id:Date.now(),path:doc.path,kind});}})),
  {id:'knowledge.obsidian',title:'在 Obsidian 中打开当前笔记',category:'知识库',enabled:!!doc&&!busy,availabilityReason:'请先打开笔记',execute:()=>openObsidian()},
 ]);
 const title=doc?.path.split('/').at(-1)??'选择一篇笔记';
 if(!settings?.vaultId)return <section className="knowledge-start"><div className="knowledge-start-icon"><BookOpen size={32}/></div><h2>知识归你，笔记留在本地</h2><p>创建 Markdown 知识库，或连接已有的 Obsidian Vault。无需安装 Obsidian 即可使用。</p><div><button className="primary" disabled={busy} onClick={()=>void chooseVault(true)}><Plus size={16}/>创建映流知识库</button><button className="secondary" disabled={busy} onClick={()=>void chooseVault(false)}><FolderOpen size={16}/>选择已有知识库</button></div><p className="help">已有知识库中的映流内容保存在“映流”文件夹。不会修改 Obsidian 配置。</p>{error&&<p className="error-text" role="alert">{error}</p>}</section>;
 return <section className="knowledge-workspace" data-command-scope="knowledge" aria-label="Markdown知识库" style={{"--knowledge-tree-width":`${layout.treeWidth}px`,"--knowledge-info-width":`${layout.infoWidth}px`} as React.CSSProperties}>
  <div className="knowledge-toolbar"><div className="knowledge-vault-name" title={settings.vaultPath??''}><BookOpen size={17}/><strong>我的知识库</strong></div><button className="icon-button" aria-label={left?'隐藏文件树':'显示文件树'} aria-pressed={left} onClick={()=>setLeft(!left)}><PanelLeft size={17}/></button><button className="icon-button" aria-label={right?'隐藏笔记信息':'显示笔记信息'} aria-pressed={right} onClick={()=>setRight(!right)}><PanelRight size={17}/></button><button className="secondary" disabled={busy} onClick={()=>prepare('新建笔记')}><Plus size={15}/>新建笔记</button><button className="secondary" disabled={busy} onClick={()=>prepare('新建文件夹')}>新建文件夹</button><button className="icon-button" aria-label="刷新知识库" disabled={indexing} onClick={()=>void scan()}><RefreshCw size={17} className={indexing?'spin':''}/></button><button className="text-button" disabled={busy} onClick={()=>openObsidian(false)}>配置 Obsidian 联动</button><button className="text-button" disabled={busy} onClick={()=>guard(()=>{void chooseVault(false);})}>切换知识库</button></div>
  {error&&<p role="alert" className="error-text knowledge-warning">{error}</p>}
  <div className={`knowledge-layout ${left?'':'without-tree'} ${right?'':'without-info'}`}>
   {left&&<aside className="knowledge-tree-panel"><div className="knowledge-pane-title">文件</div><FileTree key={settings.vaultId} vaultId={settings.vaultId} selected={doc?.path??null} revision={revision} onOpen={openNote} onAction={treeAction}/></aside>}
   {left&&<ResizeHandle label="知识文件树宽度" value={layout.treeWidth} min={160} max={480} onChange={treeWidth=>changeLayout({treeWidth})}/>}<div className="knowledge-center"><div className="knowledge-search"><input aria-label="搜索知识库" placeholder="搜索笔记、标签或视频转录" maxLength={200} value={query} onChange={e=>{setQuery(e.target.value);setSearchPage(1);}}/>{tag&&<button className="text-button" onClick={()=>setTag('')}>#{tag} ×</button>}</div>
    {results&&<div className="knowledge-search-results" aria-label="知识库搜索结果"><p className="help">找到 {results.total} 条</p>{results.items.map((hit,i)=><button key={`${hit.path??hit.analysisId}-${hit.startMs??i}`} onClick={()=>hit.kind==='note'&&hit.path?openNote(hit.path):hit.itemId&&onOpenVideo?guard(()=>onOpenVideo(hit.itemId!,hit.startMs??0)):notify('该转录尚未关联可播放课程')}><strong>{hit.title}</strong><span>{hit.kind==='transcript'?'视频转录 · ':''}{hit.excerpt}</span></button>)}<Pages page={searchPage} total={results.total} setPage={setSearchPage}/></div>}
    <div className="knowledge-tabs" onKeyDown={e=>{if(!["ArrowLeft","ArrowRight","Home","End"].includes(e.key)||(e.target as HTMLElement).getAttribute("role")!=="tab")return;const nodes=[...e.currentTarget.querySelectorAll<HTMLButtonElement>("button[role=tab]")],index=nodes.indexOf(e.target as HTMLButtonElement),next=e.key==="Home"?0:e.key==="End"?nodes.length-1:(index+(e.key==="ArrowLeft"?-1:1)+nodes.length)%nodes.length;e.preventDefault();nodes[next]?.focus();nodes[next]?.click();}} role="tablist" aria-label="打开的笔记">{tabs.filter(s=>s.vaultId===settings.vaultId).map(s=><div key={s.key} className={activeKey===s.key?'active':''}><button role="tab" aria-selected={activeKey===s.key} title={s.path} onClick={()=>{void read(s.path);}}>{s.path.split('/').at(-1)}{sessions.dirty(s.key)?' •':''}</button><button className="icon-button" aria-label={`关闭笔记 ${s.path}`} onClick={()=>guard(()=>{void closeTab(s.key);})}>×</button></div>)}</div><header className="knowledge-note-header"><div><strong title={doc?.path}>{title}</strong><span role="status">{busy||active?.state==='saving'?'正在保存…':active?.state==='conflict'?'外部修改冲突':active?.state==='error'?'保存失败，草稿保留':dirty?'等待自动保存':'已保存到本地'}</span></div><div role="group" aria-label="笔记显示模式">{(['编辑','富文本','预览','分屏']as const).map(m=><button key={m} className={mode===m?'active':''} aria-pressed={mode===m} onClick={()=>setMode(m)}>{m}</button>)}</div></header>
    {doc?<><div className="knowledge-note-actions"><button className="primary" disabled={!dirty||busy} onClick={()=>void save()}><Save size={14}/>保存</button><button className="text-button" disabled={busy} onClick={()=>prepare('重命名笔记')}>重命名</button><button className="text-button" disabled={busy} onClick={()=>prepare('移动笔记')}>移动</button><button className="text-button" disabled={busy} onClick={()=>prepare('删除笔记')}>删除</button><button className="text-button" disabled={busy} onClick={()=>openObsidian()}><ArrowUpRight size={14}/>在Obsidian中打开</button></div><NoteTools key={activeKey} vaultId={settings.vaultId} path={doc.path} text={text} revision={revision} prepare={prepareNote} apply={applyNote} openNote={openNote} created={newPath=>{setRevision(v=>v+1);openNote(newPath);}} jump={jump} request={toolRequest}/><div className={mode==='分屏'?'knowledge-editor-split':'knowledge-editor-single'}>{mode==='富文本'&&rich?.mode==='rich'?<Suspense fallback={<p role="status">正在打开富文本…</p>}><RichTextEditor value={text} onChange={setText} readOnly={busy}/></Suspense>:mode!=='预览'&&<MarkdownEditor key={`${settings.vaultId}:${doc.path}`} value={text} onChange={setText} readOnly={busy} editorRef={editor}/ >}{mode==='富文本'&&rich?.mode==='source'&&<p role="status" className="help">已回退源码编辑，原文保持完整：{rich.reason}</p>}{(mode==='预览'||mode==='分屏')&&<MarkdownPreview value={text} vaultId={settings.vaultId} path={doc.path} onLink={(target,wiki)=>void link(target,wiki)} onExternal={url=>{void invoke('open_source',{url}).catch(e=>notify(aiError(e)));}} onVideo={onOpenVideo?(item,time)=>guard(()=>onOpenVideo(item,time)):undefined}/>}</div></>:<div className="knowledge-no-note"><FileText size={32}/><h3>从文件树打开笔记</h3><p>或新建一篇笔记，记录你的学习收获。</p></div>}
   </div>
   {right&&<ResizeHandle label="知识信息栏宽度" reverse value={layout.infoWidth} min={200} max={600} onChange={infoWidth=>changeLayout({infoWidth})}/>} {right&&<aside className="knowledge-info"><div className="knowledge-pane-title">笔记信息</div><h3>大纲</h3><nav aria-label="笔记大纲">{outline.map(h=><button key={h.id} style={{paddingLeft:8+(h.depth-1)*10}} onClick={()=>jump(h.id)}>{h.text}</button>)}</nav>{text.length>MAX_OUTLINE_CHARACTERS&&<p className="help">笔记较大，暂不生成大纲，仍可编辑和搜索。</p>}<h3>标签</h3><div className="knowledge-tags">{tags.length?tags.map(t=><button aria-pressed={tag===t} key={t} onClick={()=>{setTag(tag===t?'':t);setSearchPage(1);}}>#{t}</button>):<p className="help">暂无标签</p>}</div><h3>反向链接</h3>{backlinks?.items.map(b=><button className="knowledge-backlink" key={b.path} onClick={()=>openNote(b.path)}>{b.title}</button>)}{backlinks&&!backlinks.total&&<p className="help">暂无引用此笔记的内容</p>}{backlinks&&<Pages page={backlinkPage} total={backlinks.total} setPage={setBacklinkPage}/>}</aside>}
  </div>
  {pending&&<LearningDialog title="未保存的修改" close={()=>setPending(null)} busy={busy}><p>打开的笔记中有未保存内容，离开前如何处理？</p><footer><button className="secondary" onClick={()=>setPending(null)}>继续编辑</button><button className="secondary" onClick={()=>{void discardAndContinue();}}>放弃修改</button><button className="primary" disabled={busy} onClick={()=>{void saveAndContinue();}}>保存后继续</button></footer></LearningDialog>}
  {conflict&&<LearningDialog title="文件已在其他程序中修改" close={()=>{setConflict(null);setPending(null);}} busy={busy}><p>磁盘版本已更新。你的编辑内容仍保留，请选择如何处理。</p><button className="text-button" onClick={()=>setExternalView(!externalView)}>查看外部版本</button>{externalView&&<pre className="knowledge-conflict-text">{conflict.text}</pre>}<footer><button className="secondary" onClick={()=>{setConflict(null);setPending(null);}}>继续编辑</button><button className="secondary" onClick={()=>{if(activeKey)sessions.reload(activeKey,conflict);setConflict(null);setPending(null);}}>重新加载外部版本</button><button className="primary" disabled={busy} onClick={()=>{void save(conflict.hash).then(ok=>{if(ok&&pending){const next=pending;setPending(null);next();}});}}>以当前内容覆盖</button></footer></LearningDialog>}
  {action&&<LearningDialog title={action} close={()=>setAction(null)} busy={busy}><form onSubmit={e=>{e.preventDefault();void mutate();}}>{action.startsWith('删除')?<p>{action==='删除空笔记本'?`确认删除空笔记本“${actionSource?.path}”？非空目录会被拒绝。`:`将“${actionSource?.path}”移至知识库回收目录。关联视频会保留。`}</p>:<label>相对路径<input autoFocus required value={path} onChange={e=>setPath(e.target.value)} placeholder="映流/课程/笔记.md"/></label>}{actionError&&<p className="error-text" role="alert">{actionError}</p>}<footer><button type="button" className="secondary" disabled={busy} onClick={()=>setAction(null)}>取消</button><button className="primary" disabled={busy}>{action.startsWith('新建')?'创建':action==='删除空笔记本'?'确认删除空笔记本':action==='删除笔记'?'移至回收目录':action==='复制笔记'?'创建副本':'保存位置'}</button></footer></form></LearningDialog>}
  {choices&&<LearningDialog title="选择关联笔记" close={()=>setChoices(null)}>{choices.map(c=><button className="knowledge-link-choice" key={c.path} onClick={()=>{setChoices(null);openNote(c.path);}}>{c.title}<small>{c.path}</small></button>)}</LearningDialog>}
  {obsidianDirty!==null&&<LearningDialog title="打开 Obsidian 前处理未保存内容" close={()=>setObsidianDirty(null)} busy={busy}><p>Obsidian 将读取磁盘中的笔记。可以先保存，或打开磁盘版本并保留当前编辑。</p><footer><button className="secondary" disabled={busy} onClick={()=>setObsidianDirty(null)}>继续编辑</button><button className="secondary" disabled={busy} onClick={()=>{setObsidianDirty(null);setObsidian({path:current.current.doc?.path??null,autoOpen:obsidianDirty===true});}}>打开磁盘版本，保留编辑</button><button className="primary" disabled={busy} onClick={()=>{setObsidianDirty(null);void save().then(ok=>{if(ok&&alive.current)setObsidian({path:current.current.doc?.path??null,autoOpen:obsidianDirty===true});});}}>保存后打开</button></footer></LearningDialog>}
  {obsidian&&<ObsidianConnection key={`${settings.vaultId}:${obsidian.path??''}`} settings={settings} path={obsidian.path} autoOpen={obsidian.autoOpen} close={()=>setObsidian(null)} notify={notify} onSettings={next=>{if(valid(next.vaultId??'')){current.current={...current.current,settings:next};setSettings(next);}}}/>}
 </section>;
}
function Pages({page,total,setPage}:{page:number;total:number;setPage:(page:number)=>void}){return total>50?<div className="knowledge-pages"><button className="text-button" disabled={page<=1} onClick={()=>setPage(page-1)}>上一页</button><span>{page}/{Math.ceil(total/50)}</span><button className="text-button" disabled={page*50>=total} onClick={()=>setPage(page+1)}>下一页</button></div>:null;}
