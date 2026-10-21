import {useRegisterCommands,useCommands,useCommandSnapshot,useShortcutBindings} from './CommandProvider';
import type {CommandDescriptor} from './CommandRegistry';
import {useWorkspace} from '../workspace/WorkspaceProvider';
import {BUILTIN_WORKSPACE_PRESETS} from '../workspace/presets';
import {useFeedback} from '../../components/Feedback';
type Page='download'|'learning'|'knowledge'|'media'|'history'|'settings';
const inactive=(id:string,title:string,scope:'player'|'knowledge'|'global',shortcut?:string):CommandDescriptor=>({id,title,scope,shortcut,category:scope==='knowledge'?'知识库':'学习与播放',keywords:scope==='knowledge'?['note','knowledge','笔记']:['course','video','课程'],enabled:false,availabilityReason:scope==='knowledge'?'请先在知识库打开一篇笔记':'请先在学习库选择课程',execute:()=>{}});
const inactiveCommands:CommandDescriptor[]=[
 ...[['learning.search','搜索课程'],['learning.continue','从上次位置继续'],['learning.play-current','播放当前课程'],['learning.analyze','分析当前视频'],['learning.notes','生成课程笔记'],['learning.subtitles','打开课程字幕'],['learning.ai','打开 AI 学习面板']].map(([id,title])=>inactive(id,title,'global')),
 ...[['knowledge.new','新建笔记','Ctrl+Shift+N'],['knowledge.search','搜索笔记'],['knowledge.save','保存当前笔记','Ctrl+S'],['knowledge.mode','切换实时预览与源码'],['knowledge.recent','打开最近笔记'],['knowledge.template','创建模板笔记'],['knowledge.history','打开版本历史'],['knowledge.obsidian','在 Obsidian 中打开当前笔记'],['knowledge.insert-time','插入视频时间点']].map(([id,title,shortcut])=>({...inactive(id,title,'knowledge',shortcut),allowInEditable:id==='knowledge.save'})),
 ...[['player.toggle','播放 / 暂停','Space'],['player.seek-back','后退5秒','J'],['player.seek-forward','前进5秒','L'],['player.fullscreen','切换全屏','F11'],['player.seek-back-arrow','后退5秒','ArrowLeft'],['player.seek-forward-arrow','前进5秒','ArrowRight'],['player.seek-back-large','后退10秒','Shift+ArrowLeft'],['player.seek-forward-large','前进10秒','Shift+ArrowRight'],['player.mute','切换静音','M'],['player.fullscreen-legacy','切换全屏','F'],['player.next','下一课','N'],['player.previous','上一课','P'],['player.exit-fullscreen','退出全屏','Escape'],['player.copy-time','复制当前时间点'],['player.create-note','创建时间点笔记'],['player.load-subtitle','加载字幕'],['player.mark-complete','切换课程完成状态'],['player.layout-standard','标准学习布局'],['player.layout-stacked','上下布局'],['player.layout-theater','剧院模式'],['player.layout-focus','专注播放']].map(([id,title,shortcut])=>inactive(id,title,'player',shortcut)),
];
export function AppCommands({navigate,focusDownloads,openDirectory,openRecent,hasRecent}:{navigate:(page:Page)=>void;focusDownloads:()=>void;openDirectory:()=>Promise<unknown>;openRecent:()=>Promise<unknown>;hasRecent:boolean}){
 const {workspace,setLayout,applyPreset,setAppearance}=useWorkspace(),{openHelp,openPalette,openSearch,shortcuts}=useCommands(),{undo,canUndo}=useFeedback();
 const snapshot=useCommandSnapshot();useShortcutBindings();const binding=(id:string)=>{const command=snapshot.commands.find(c=>c.id===id);return command?shortcuts.binding(command):null;};
 useRegisterCommands(inactiveCommands);
 const presets=[...BUILTIN_WORKSPACE_PRESETS(),...workspace.presets];
 useRegisterCommands([
  ...(['download','learning','knowledge','media','history','settings'] as Page[]).map((page,i)=>({id:`navigate.${page}`,title:`打开${['下载','学习库','知识库','媒体工具','历史','设置'][i]}`,category:'全局导航',keywords:[page,'导航'],execute:()=>navigate(page)})),
  {id:'download.focus',title:'粘贴下载链接',category:'下载',keywords:['link','url','链接'],shortcut:'Ctrl+L',allowInEditable:true,execute:focusDownloads},
  {id:'file.download-directory',title:'打开下载目录',category:'文件',keywords:['folder','directory'],execute:openDirectory},
  {id:'file.recent',title:'打开最近文件',category:'文件',enabled:hasRecent,availabilityReason:'尚无已完成的下载文件',execute:openRecent},
  {id:'workspace.compact',title:'切换紧凑模式',category:'工作区',keywords:['compact','密度'],execute:()=>setAppearance(old=>({...old,density:old.density==='compact'?'standard':'compact'}))},
  {id:'workspace.sidebar',title:'折叠或展开主导航',category:'工作区',keywords:['sidebar','navigation','侧边栏'],execute:()=>setAppearance(old=>({...old,sidebarCollapsed:!old.sidebarCollapsed}))},
  {id:'workspace.focus',title:'切换专注模式',category:'工作区',execute:()=>setLayout(old=>({...old,player:{...old.player,layout:old.player.layout==='focus'?'standard':'focus'}}))},
  {id:'workspace.default',title:'恢复默认布局',category:'工作区',description:'只恢复界面布局，保留所有课程与笔记',execute:()=>applyPreset('builtin.default')},
  ...presets.map(p=>({id:`workspace.preset.${p.id}`,title:`应用布局：${p.name}`,category:'工作区',keywords:['preset','layout','预设'],execute:()=>applyPreset(p.id)})),
  {id:'app.undo-operation',title:'撤销上次业务操作',category:'操作',shortcut:'Ctrl+Alt+Z',enabled:canUndo,availabilityReason:'暂无可撤销的业务操作，正文撤销请使用 Ctrl+Z',execute:undo},
 ]);
 return <div className="global-command-tools"><button className="secondary" onClick={openPalette} title="搜索并执行命令">命令面板 <kbd>{binding('app.palette')??'未绑定'}</kbd></button><button className="text-button" onClick={openSearch} title="搜索课程与知识笔记">搜索 <kbd>{binding('app.search')??'未绑定'}</kbd></button><button className="text-button" onClick={openHelp}>快捷键</button></div>;
}
