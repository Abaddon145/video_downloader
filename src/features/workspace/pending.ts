import {readWorkspace} from './defaults.ts';
import type {WorkspaceSettings} from './types.ts';
const key='yingliu-workspace-pending-v1';
type Store=Pick<Storage,'getItem'|'setItem'|'removeItem'>;
export function readPendingWorkspace(store:Store):WorkspaceSettings|null {
 const raw=store.getItem(key);
 if(!raw||raw.length>131072)return null;
 try {
  const original:unknown=JSON.parse(raw),settings=readWorkspace(original);
  return JSON.stringify(settings)===JSON.stringify(original)?settings:null;
 }catch{return null;}
}
export function writePendingWorkspace(store:Store,settings:WorkspaceSettings){store.setItem(key,JSON.stringify(settings));}
export function acknowledgeWorkspace(store:Store,settings:WorkspaceSettings){
 if(store.getItem(key)===JSON.stringify(settings))store.removeItem(key);
}
