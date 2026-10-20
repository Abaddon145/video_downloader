import {createContext,useCallback,useContext,useEffect,useLayoutEffect,useMemo,useRef,useState,useSyncExternalStore} from 'react';
import type {ReactNode} from 'react';
import {CommandRegistry} from './CommandRegistry';
import type {CommandDescriptor,CommandStorage} from './CommandRegistry';
import {ShortcutManager} from './ShortcutManager';
import {CommandPalette} from './CommandPalette';
import {ShortcutSettings} from './ShortcutSettings';
import './commands.css';
interface CommandContextValue {
 registry:CommandRegistry;shortcuts:ShortcutManager;overlay:'palette'|'help'|null;
 run:CommandRegistry['run'];register:CommandRegistry['register'];openPalette:()=>void;openSearch:()=>void;openHelp:()=>void;closeOverlay:()=>void;closePalette:()=>void;
}
const CommandContext=createContext<CommandContextValue|null>(null);
export function CommandProvider({children,onOpenSearch}:{children:ReactNode;onOpenSearch?:()=>void}){
 const [overlay,setOverlay]=useState<'palette'|'help'|null>(null);
 const [services]=useState(()=>{let storage:CommandStorage|undefined;try{storage=localStorage;}catch{}const registry=new CommandRegistry(storage);return {registry,shortcuts:new ShortcutManager(registry,storage)};});
 const openPalette=useCallback(()=>setOverlay(previous=>previous==='palette'?null:'palette'),[]);
 const openHelp=useCallback(()=>setOverlay('help'),[]),closeOverlay=useCallback(()=>setOverlay(null),[]);
 const closePalette=useCallback(()=>setOverlay(previous=>previous==='palette'?null:previous),[]);
 const currentSearch=useRef(onOpenSearch);useLayoutEffect(()=>{currentSearch.current=onOpenSearch;},[onOpenSearch]);
 const openSearch=useCallback(()=>{if(currentSearch.current){setOverlay(null);currentSearch.current();}},[]);
 useEffect(()=>services.shortcuts.attach(window),[services]);
 useEffect(()=>services.registry.register([
  {id:'app.palette',title:'打开命令面板',category:'全局',keywords:['command','palette','命令'],shortcut:'Ctrl+K',allowInEditable:true,execute:openPalette},
  {id:'app.search',title:'搜索课程与笔记',category:'全局',keywords:['search','搜索','course','note'],shortcut:'Ctrl+P',allowInEditable:true,enabled:!!onOpenSearch,availabilityReason:'搜索入口尚未连接',execute:openSearch},
  {id:'app.shortcuts',title:'快捷键帮助与设置',category:'全局',keywords:['help','keyboard','shortcut','帮助'],shortcut:'Ctrl+Alt+K',execute:openHelp},
 ]),[services,openPalette,openSearch,openHelp,!!onOpenSearch]);
 const value=useMemo(()=>({...services,overlay,run:services.registry.run,register:services.registry.register,openPalette,openSearch,openHelp,closeOverlay,closePalette}),[services,overlay,openPalette,openSearch,openHelp,closeOverlay,closePalette]);
 return <CommandContext.Provider value={value}>{children}<CommandPalette/><ShortcutSettings/></CommandContext.Provider>;
}
export function useCommands(){const context=useContext(CommandContext);if(!context)throw new Error('Commands require CommandProvider');return context;}
export function useCommandSnapshot(){const {registry}=useCommands();return useSyncExternalStore(registry.subscribe,registry.getSnapshot,registry.getSnapshot);}
export function useShortcutBindings(){const {shortcuts}=useCommands();return useSyncExternalStore(shortcuts.subscribe,shortcuts.getSnapshot,shortcuts.getSnapshot);}
export function useRegisterCommands(commands:CommandDescriptor[]){
 const {registry}=useCommands(),current=useRef(commands);
 useLayoutEffect(()=>{current.current=commands;});
 // Callbacks read the latest committed target; callback identity does not re-register commands.
 const metadata=JSON.stringify(commands.map(({execute,...descriptor})=>descriptor));
 useLayoutEffect(()=>registry.register(commands.map(command=>({...command,execute:()=>current.current.find(c=>c.id===command.id)?.execute()}))),[registry,metadata]);
}

