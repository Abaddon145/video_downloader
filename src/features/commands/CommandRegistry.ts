export type CommandScope='global'|'player'|'knowledge';
export interface CommandDescriptor {
 id:string;title:string;category:string;description?:string;keywords?:string[];enabled?:boolean;availabilityReason?:string;
 shortcut?:string|null;scope?:CommandScope;allowInEditable?:boolean;execute:()=>unknown|Promise<unknown>;
}
export interface CommandRecent {id:string;time:number;count:number}
export interface CommandSnapshot {commands:CommandDescriptor[];recent:CommandRecent[];running:string[];error:string}
export type CommandResult={ok:true}|{ok:false;error:string};
export interface CommandStorage {getItem:(key:string)=>string|null;setItem:(key:string,value:string)=>void}
const RECENT_KEY='yingliu.commands.recent.v1';
export class CommandRegistry {
 private registrations=new Map<symbol,CommandDescriptor[]>();
 private listeners=new Set<()=>void>();
 private running=new Set<string>();
 private snapshot:CommandSnapshot={commands:[],recent:[],running:[],error:''};
 private storage?:CommandStorage;
 constructor(storage?:CommandStorage){
  this.storage=storage;
  try {const raw=storage?.getItem(RECENT_KEY);if(raw&&raw.length<16384){const values:unknown=JSON.parse(raw);if(Array.isArray(values))this.snapshot.recent=values.filter((v):v is CommandRecent=>!!v&&typeof v==='object'&&Object.keys(v).every(k=>['id','time','count'].includes(k))&&typeof v.id==='string'&&/^[a-zA-Z0-9._-]{1,100}$/.test(v.id)&&Number.isFinite(v.time)&&Number.isInteger(v.count)&&v.count>0).slice(0,30);}}
  catch {/* Invalid or unavailable local preferences must not block the app. */}
 }
 subscribe=(listener:()=>void)=>{this.listeners.add(listener);return()=>{this.listeners.delete(listener);};};
 getSnapshot=()=>this.snapshot;
 private publish(patch:Partial<CommandSnapshot>={}){this.snapshot={...this.snapshot,...patch,running:[...this.running]};for(const listener of this.listeners)listener();}
 register=(commands:CommandDescriptor[])=>{
  const token=Symbol();this.registrations.set(token,commands);this.refresh();
  return()=>{if(this.registrations.delete(token))this.refresh();};
 };
 private refresh(){const commands=new Map<string,CommandDescriptor>();for(const group of this.registrations.values())for(const command of group)commands.set(command.id,command);this.publish({commands:[...commands.values()],error:''});}
 clearError=()=>this.publish({error:''});
 run=async(id:string):Promise<CommandResult>=>{
  const command=this.snapshot.commands.find(c=>c.id===id);
  const unavailable=!command?'此命令当前不可用':command.enabled===false?command.availabilityReason||'此命令当前不可用':this.running.has(id)?'命令正在执行':null;
  if(unavailable){this.publish({error:unavailable});return {ok:false,error:unavailable};}
  this.running.add(id);this.publish({error:''});
  try {
   await command!.execute();
   if(['app.palette','app.search','app.shortcuts'].includes(id))return {ok:true};
   const previous=this.snapshot.recent.find(r=>r.id===id);const recent=[{id,time:Date.now(),count:(previous?.count??0)+1},...this.snapshot.recent.filter(r=>r.id!==id)].slice(0,30);
   try{this.storage?.setItem(RECENT_KEY,JSON.stringify(recent));}catch{/* Recents are optional; the completed action remains successful. */}
   this.publish({recent});return {ok:true};
  }catch(error){const message=error instanceof Error?error.message:String(error);if(this.snapshot.commands.find(c=>c.id===id)===command)this.publish({error:message});return {ok:false,error:message};}
  finally{this.running.delete(id);this.publish();}
 };
}
export function searchCommands(commands:CommandDescriptor[],query:string,recent:CommandRecent[]=[],category=''){
 const terms=query.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
 const score=(command:CommandDescriptor)=>{
  const title=command.title.toLocaleLowerCase(),haystack=[title,command.id,command.category,command.description??'',...(command.keywords??[])].join(' ').toLocaleLowerCase();let total=0;
  for(const term of terms){if(title===term)total+=100;else if(title.includes(term))total+=60;else if(haystack.includes(term))total+=35;else {let at=0;for(const character of term){at=haystack.indexOf(character,at);if(at<0)return -1;at++;}total+=5;}}
  const used=recent.find(r=>r.id===command.id);return total+(terms.length?Math.min(used?.count??0,10):used?1000+30-recent.indexOf(used)+Math.min(used.count,20):0);
 };
 return commands.filter(c=>!category||c.category===category).map((command,index)=>({command,index,score:score(command)})).filter(c=>c.score>=0).sort((a,b)=>b.score-a.score||a.index-b.index).map(c=>c.command);
}

