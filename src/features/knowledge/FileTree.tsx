import {useEffect,useState,type MouseEvent,type KeyboardEvent} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {ChevronDown,ChevronRight,FileText,Folder} from 'lucide-react';
import {ContextMenu,useContextMenu,type ContextMenuItem} from '../../components/ContextMenu';
import type {KnowledgeView,TreeEntry,TreePage} from '../../types/knowledge';
import {aiError} from '../ai/lib';
type Trigger=MouseEvent<HTMLElement>|KeyboardEvent<HTMLElement>;
type MenuTarget={entry:TreeEntry;toggle?:()=>void};
type TreeProps={vaultId:string;selected:string|null;revision:number;onOpen:(path:string)=>void;onAction:(entry:TreeEntry,action:string)=>void|Promise<unknown>};
type DirectoryProps=Omit<TreeProps,'onAction'>&{parent:string;sort:string;onMenu:(event:Trigger,target:MenuTarget)=>void};
export function FileTree({vaultId,selected,revision,onOpen,onAction}:TreeProps){
 const {menu,open,close}=useContextMenu<MenuTarget>(),[sort,setSort]=useState('name');
 const items:ContextMenuItem[]=menu?.target.entry.directory?[
  {id:'new-note',label:'新建笔记',group:'create'},{id:'new-folder',label:'新建子笔记本',group:'create'},
  {id:'rename',label:'重命名',group:'edit'},{id:'move',label:'移动',group:'edit'},
  {id:'sort',label:'排序',group:'view',children:[{id:'sort-name',label:'按名称升序'},{id:'sort-nameDesc',label:'按名称降序'}]},
  {id:'toggle',label:'展开 / 折叠',group:'view'},
  {id:'delete-empty',label:'删除空笔记本',danger:true,group:'remove'}
 ]:[
  {id:'open',label:'打开',group:'open'},{id:'open-new-tab',label:'在新标签页打开',group:'open'},
  {id:'favorite',label:'切换收藏',group:'knowledge'},{id:'history',label:'版本历史',group:'knowledge'},{id:'ai',label:'AI整理笔记',group:'knowledge'},
  {id:'rename',label:'重命名',group:'edit'},{id:'move',label:'移动',group:'edit'},{id:'duplicate',label:'复制笔记',group:'edit'},
  {id:'export',label:'导出 Markdown',group:'file'},
  {id:'obsidian',label:'在 Obsidian 中打开',group:'obsidian'},{id:'configure-obsidian',label:'配置 Obsidian 联动',group:'obsidian'},
  {id:'folder',label:'在资源管理器中显示',group:'file'},
  {id:'delete',label:'移入回收站',danger:true,group:'remove'}
 ];
 return <div className="knowledge-file-tree" aria-label="知识库文件树"><Directory vaultId={vaultId} parent="" selected={selected} revision={revision} onOpen={onOpen} onMenu={open} sort={sort}/>
  <ContextMenu menu={menu} items={items} onClose={close} onAction={(item,target)=>{if(item.id==='toggle')target.toggle?.();else if(item.id.startsWith('sort-'))setSort(item.id.slice(5));else return onAction(target.entry,item.id);}}/>
 </div>;
}
function Directory({vaultId,parent,selected,revision,onOpen,onMenu,sort}:DirectoryProps){
 const [data,setData]=useState<TreePage|null>(null),[page,setPage]=useState(1),[error,setError]=useState('');
 useEffect(()=>setPage(1),[sort]);
 useEffect(()=>{let alive=true;setError('');invoke<KnowledgeView<TreePage>>('knowledge_tree',{vaultId,parent,page,sort}).then(v=>{if(alive&&v.vaultId===vaultId){const last=Math.max(1,Math.ceil(v.data.total/50));if(page>last)setPage(last);else setData(v.data);setError('');}}).catch(e=>{if(alive)setError(aiError(e));});return()=>{alive=false;};},[vaultId,parent,page,revision,sort]);
 return <>{error&&<p className="error-text" role="alert">{error}</p>}{data?.skipped?<p className="help">部分文件被占用或不可读取。</p>:null}{data?.items.map(item=>item.directory?<FolderRow key={item.path} item={item} vaultId={vaultId} selected={selected} revision={revision} onOpen={onOpen} onMenu={onMenu} sort={sort}/>:<button className={selected===item.path?'knowledge-tree-note active':'knowledge-tree-note'} key={item.path} aria-label={`打开笔记 ${item.name}`} aria-current={selected===item.path?'page':undefined} onClick={()=>onOpen(item.path)} onContextMenu={e=>onMenu(e,{entry:item})} onKeyDown={e=>onMenu(e,{entry:item})}><FileText size={14} aria-hidden="true"/><span>{item.name}</span></button>)}{data&&!data.items.length&&<p className="help">暂无笔记</p>}{data&&data.total>50&&<div className="knowledge-tree-pages"><button type="button" className="text-button" disabled={page<=1} onClick={()=>setPage(page-1)}>上一页文件</button><span>{page}/{Math.ceil(data.total/50)}</span><button type="button" className="text-button" disabled={page*50>=data.total} onClick={()=>setPage(page+1)}>下一页文件</button></div>}</>;
}
function FolderRow({item,vaultId,selected,revision,onOpen,onMenu,sort}:Omit<DirectoryProps,'parent'>&{item:TreeEntry}){
 const [expanded,setExpanded]=useState(false);const toggle=()=>setExpanded(old=>!old);
 return <div className="knowledge-tree-folder"><button type="button" aria-label={`${expanded?'收起':'展开'}文件夹 ${item.name}`} aria-expanded={expanded} onClick={toggle} onContextMenu={e=>onMenu(e,{entry:item,toggle})} onKeyDown={e=>onMenu(e,{entry:item,toggle})}>{expanded?<ChevronDown size={13} aria-hidden="true"/>:<ChevronRight size={13} aria-hidden="true"/>}<Folder size={14} aria-hidden="true"/><span>{item.name}</span></button>{expanded&&<div className="knowledge-tree-children"><Directory vaultId={vaultId} parent={item.path} selected={selected} revision={revision} onOpen={onOpen} onMenu={onMenu} sort={sort}/></div>}</div>;
}
