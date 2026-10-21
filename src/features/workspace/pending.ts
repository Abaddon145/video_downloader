import {validateWorkspace} from './defaults.ts';
import type {WorkspaceSettings} from './types.ts';
const key='yingliu-workspace-pending-v1';
type Store=Pick<Storage,'getItem'|'setItem'|'removeItem'>;
export function readPendingWorkspace(store:Store):WorkspaceSettings|null {
 const raw=store.getItem(key);
 if(!raw||raw.length>131072)return null;
 try {
  return validateWorkspace(JSON.parse(raw));
 }catch{return null;}
}
export function writePendingWorkspace(store:Store,settings:WorkspaceSettings){store.setItem(key,JSON.stringify(settings));}
export function acknowledgeWorkspace(store:Store,settings:WorkspaceSettings){
 const raw=store.getItem(key);
 if(!raw)return;
 try{if(JSON.stringify(validateWorkspace(JSON.parse(raw)))===JSON.stringify(settings))store.removeItem(key);}catch{/* Invalid recovery data remains untouched. */}
}
