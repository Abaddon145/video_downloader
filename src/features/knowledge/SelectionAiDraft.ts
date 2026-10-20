import type {SelectionSnapshot} from './SelectionToolbar';
import type {NoteSnapshot} from './NoteTools';

export function selectionMatches(expected:NoteSnapshot,current:NoteSnapshot|null):boolean{
 return !!current&&current.vaultId===expected.vaultId&&current.path===expected.path&&current.hash===expected.hash&&current.editVersion===expected.editVersion&&current.text===expected.text;
}

export function applySelectionDraft(snapshot:SelectionSnapshot,replacement:string,current:string):string|null{
 if(current!==snapshot.source||snapshot.from<0||snapshot.to>current.length||snapshot.from>=snapshot.to||current.slice(snapshot.from,snapshot.to)!==snapshot.selection)return null;
 return current.slice(0,snapshot.from)+replacement+current.slice(snapshot.to);
}
