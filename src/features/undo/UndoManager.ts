export interface UndoEntry {label:string;undo:()=>Promise<void>}
export class UndoManager {
 private entries:UndoEntry[]=[];
 private running=false;
 private listeners=new Set<()=>void>();
 private limit:number;
 constructor(limit=30){this.limit=limit;}
 subscribe=(fn:()=>void)=>{this.listeners.add(fn);return()=>{this.listeners.delete(fn);};};
 private changed(){for(const fn of this.listeners)fn();}
 get size(){return this.entries.length;}
 get busy(){return this.running;}
 peek=()=>this.entries.at(-1)??null;
 push(entry:UndoEntry){this.entries.push(entry);if(this.entries.length>this.limit)this.entries.shift();this.changed();}
 async undo(){
  const entry=this.peek();if(!entry||this.running)return false;
  this.running=true;this.changed();
  try{await entry.undo();const index=this.entries.indexOf(entry);if(index>=0)this.entries.splice(index,1);return true;}
  finally{this.running=false;this.changed();}
 }
}
