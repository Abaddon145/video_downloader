import type {KnowledgeDocument,KnowledgeWrite} from '../../types/knowledge.ts';
export interface NoteSession{key:string;vaultId:string;path:string;document:KnowledgeDocument;draft:string;editVersion:number;state:'saved'|'pending'|'saving'|'conflict'|'error'|'paused';conflict:KnowledgeDocument|null;error:string;warning:string|null}
type Writer=(vaultId:string,path:string,text:string,expectedHash:string)=>Promise<KnowledgeWrite>;

/** One queue per open document. Markdown on disk remains the authoritative saved version. */
export class NoteSessions{
 private sessions=new Map<string,NoteSession>();
 private running=new Map<string,Promise<boolean>>();
 private timers=new Map<string,ReturnType<typeof setTimeout>>();
 private listeners=new Set<()=>void>();
 private snapshot:NoteSession[]=[];
 private currentVault:string|null=null;
 private write:Writer;
 constructor(write:Writer){this.write=write;}
 key(vaultId:string,path:string){return JSON.stringify([vaultId,path]);}
 get(key:string){return this.sessions.get(key);}
 isSaving(key:string){return this.running.has(key);}
 getSnapshot=()=>this.snapshot;
 subscribe=(fn:()=>void)=>{this.listeners.add(fn);return()=>{this.listeners.delete(fn);};};
 private publish(){this.snapshot=[...this.sessions.values()].map(s=>({...s}));for(const fn of this.listeners)fn();}
 private clearTimer(key:string){const timer=this.timers.get(key);if(timer)clearTimeout(timer);this.timers.delete(key);}
 private schedule(key:string){this.clearTimer(key);const s=this.get(key);if(!s||s.conflict||s.state==='error'||s.vaultId!==this.currentVault)return;this.timers.set(key,setTimeout(()=>{this.timers.delete(key);void this.save(key);},300));}
 setVault(id:string|null){this.currentVault=id;for(const s of this.sessions.values()){this.clearTimer(s.key);if(s.vaultId!==id&&!this.running.has(s.key)&&this.dirty(s.key))s.state='paused';else if(s.vaultId===id&&s.state==='paused')s.state='pending';}this.publish();}
 open(vaultId:string,document:KnowledgeDocument){
  const key=this.key(vaultId,document.path);if(this.sessions.has(key))return key;
  if(this.sessions.size>=20)throw Error('最多打开20篇笔记，请先关闭部分标签。');
  if(this.currentVault===null)this.currentVault=vaultId;
  this.sessions.set(key,{key,vaultId,path:document.path,document:{...document},draft:document.text,editVersion:0,state:'saved',conflict:null,error:'',warning:null});this.publish();return key;
 }
 dirty(key:string){const s=this.get(key);return !!s&&(s.draft!==s.document.text||this.running.has(key));}
 dirtyKeys(vaultId?:string){return this.snapshot.filter(s=>(!vaultId||s.vaultId===vaultId)&&this.dirty(s.key)).map(s=>s.key);}
 edit(key:string,text:string){const s=this.get(key);if(!s||s.draft===text)return;s.draft=text;s.editVersion++;if(!s.conflict&&!this.running.has(key))s.state=s.vaultId===this.currentVault?'pending':'paused';s.error='';this.publish();this.schedule(key);}
 refresh(key:string,disk:KnowledgeDocument){const s=this.get(key);if(!s||this.running.has(key)||s.document.hash===disk.hash)return;if(s.draft===s.document.text){s.document={...disk};s.draft=disk.text;s.state='saved';s.conflict=null;}else{s.conflict={...disk};s.state='conflict';this.clearTimer(key);}this.publish();}
 reload(key:string,disk:KnowledgeDocument){const s=this.get(key);if(!s||this.running.has(key))return false;this.clearTimer(key);s.document={...disk};s.draft=disk.text;s.editVersion++;s.conflict=null;s.error='';s.state='saved';this.publish();return true;}
 async save(key:string,overwriteHash?:string):Promise<boolean>{
  this.clearTimer(key);const existing=this.running.get(key);if(existing)return existing;
  const s=this.get(key);if(!s)return true;if(s.vaultId!==this.currentVault){s.state='paused';this.publish();return false;}
  if(s.conflict&&!overwriteHash)return false;
  if(s.draft===s.document.text&&!overwriteHash){s.state='saved';this.publish();return true;}
  // Defer the pump until the promise is registered, so synchronous writes cannot race queue ownership.
  const promise=Promise.resolve().then(async()=>{
   while(s.draft!==s.document.text||overwriteHash){
    if(s.vaultId!==this.currentVault){s.state='paused';return false;}
    const draft=s.draft,hash=overwriteHash??s.document.hash;s.state='saving';s.error='';this.publish();
    try{
     const result=await this.write(s.vaultId,s.path,draft,hash);
     if(result.vaultId!==s.vaultId||result.outcome.document.path!==s.path)throw Error('保存回执不属于当前笔记，草稿已保留。');
     if(result.outcome.status==='conflict'){s.conflict=result.outcome.document;s.state='conflict';return false;}
     s.document={...result.outcome.document};s.conflict=null;s.warning=result.indexWarning;overwriteHash=undefined;
    }catch(e){s.error=String(e);s.state='error';return false;}
   }
   s.state='saved';return true;
  }).finally(()=>{this.running.delete(key);this.publish();});
  this.running.set(key,promise);return promise;
 }
 async flush(vaultId?:string){const keys=this.dirtyKeys(vaultId);const results=await Promise.all(keys.map(key=>this.save(key)));return results.every(Boolean);}
 async settle(){await Promise.all(this.running.values());}
 close(key:string,discard=false){if(this.running.has(key)||(!discard&&this.dirty(key)))return false;this.clearTimer(key);this.sessions.delete(key);this.publish();return true;}
 discard(key:string){const s=this.get(key);return !!s&&this.reload(key,s.document);}
 move(vaultId:string,from:string,to:string){
  const moving=[...this.sessions.values()].filter(s=>s.vaultId===vaultId&&(s.path===from||s.path.startsWith(from+'/')));
  this.checkMove(vaultId,from,to);
  for(const s of moving){this.sessions.delete(s.key);s.path=to+s.path.slice(from.length);s.key=this.key(vaultId,s.path);s.document={...s.document,path:s.path};this.sessions.set(s.key,s);}this.publish();
 }
 checkMove(vaultId:string,from:string,to:string){
  const moving=[...this.sessions.values()].filter(s=>s.vaultId===vaultId&&(s.path===from||s.path.startsWith(from+'/')));
  if(moving.some(s=>this.dirty(s.key)))throw Error('移动前请先保存相关笔记。');
  for(const s of moving){const path=to+s.path.slice(from.length),key=this.key(vaultId,path);if(this.sessions.has(key)&&!moving.some(x=>x.key===key))throw Error('目标路径已有打开的标签，原标签已保留。');}
 }
 dispose(){for(const key of this.timers.keys())this.clearTimer(key);this.listeners.clear();}
}
