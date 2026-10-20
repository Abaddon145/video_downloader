import type {CommandDescriptor} from '../commands/CommandRegistry';
import {normalizeShortcut,shortcutIssue} from '../commands/ShortcutManager.ts';
const names:Record<string,string>={Space:'SPACE',ArrowLeft:'LEFT',ArrowRight:'RIGHT',ArrowUp:'UP',ArrowDown:'DOWN',Escape:'ESC',Enter:'ENTER',Tab:'TAB',Home:'HOME',End:'END',Backspace:'BS',Delete:'DEL',PageUp:'PGUP',PageDown:'PGDWN'};
export function nativeBindings(commands:CommandDescriptor[]){
 return commands.filter(c=>c.enabled!==false&&c.shortcut&&c.scope!=='knowledge'&&!shortcutIssue(c.shortcut,c.scope,commands,c.id)).flatMap(c=>{
  const normalized=normalizeShortcut(c.shortcut!);if(!normalized)return [];
  let parts=normalized.split('+');const key=parts.pop()!,shift=parts.includes('Shift');let native=names[key];
  if(!native){parts=parts.filter(p=>p!=='Shift');native=key==='Plus'?'+':/^[0-9]$/.test(key)&&shift?')!@#$%^&*('[Number(key)]:shift?key:key.toLowerCase();}
  return [{id:c.id,key:[...parts,native].join('+')}];
 });
}
