import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {KnowledgeSettings,ObsidianBinding,ObsidianConnection as Connection,ObsidianKind,ObsidianOpenResult} from '../../types/knowledge';
import {LearningDialog} from '../learning/Editors';
import {aiError} from '../ai/lib';

const states={unconfigured:'未配置',protocolUnavailable:'Obsidian URI 不可用',registrationPending:'待确认 Vault 已在 Obsidian 登记',verificationPending:'已配置，等待实际打开验证',userConfirmed:'用户确认联动正常',launchFailed:'打开请求失败',staleVault:'知识库根目录已变化，请重新配置'};

export function ObsidianConnection({settings,path,autoOpen,close,onSettings,notify}:{settings:KnowledgeSettings;path:string|null;autoOpen:boolean;close:()=>void;onSettings:(settings:KnowledgeSettings)=>void;notify:(message:string)=>void}) {
 const id=settings.vaultId!;
 const initial=settings.obsidian?.yingliuVaultId===id?settings.obsidian:null;
 const [connection,setConnection]=useState<Connection|null>(null),[kind,setKind]=useState<ObsidianKind>(initial?.kind??'vaultId');
 const [identifier,setIdentifier]=useState(initial?.obsidianVaultIdentifier??''),[registered,setRegistered]=useState(initial?.registrationConfirmed??false);
 const [busy,setBusy]=useState(true),[error,setError]=useState(''),[message,setMessage]=useState('');
 const [accepted,setAccepted]=useState<{binding:ObsidianBinding;path:string}|null>(null);
 const alive=useRef(true),ticket=useRef(0);
 const binding=connection?.binding;
 const vaultPath=connection?.vaultPath??settings.vaultPath??'';
 const changed=!binding||binding.yingliuVaultId!==id||binding.kind!==kind||binding.obsidianVaultIdentifier!==(kind==='absolutePath'?null:identifier.trim())||binding.registrationConfirmed!==registered;
 const canTest=!!path&&!busy&&!changed&&registered&&connection?.protocolAvailable&&connection.state!=='staleVault';

 useEffect(()=>{
  alive.current=true;const own=++ticket.current;
  void (async()=>{
   try {
    const status=await invoke<Connection>('knowledge_obsidian_connection',{vaultId:id});
    if(!alive.current||ticket.current!==own||status.vaultId!==id)return;
    setConnection(status);
    if(status.lastRequest?.status==='launchFailed')setMessage(status.lastRequest.message);
    if(autoOpen&&path&&status.protocolAvailable&&status.binding?.yingliuVaultId===id&&status.binding.registrationConfirmed&&status.state!=='staleVault')await launch(status.binding,path,own);
   }catch(e){if(alive.current&&ticket.current===own)setError(aiError(e));}
   finally{if(alive.current&&ticket.current===own)setBusy(false);}
  })();
  return()=>{alive.current=false;++ticket.current;};
 },[id,path,autoOpen]);

 async function launch(expected:ObsidianBinding,note:string,own:number) {
  const result=await invoke<ObsidianOpenResult>('knowledge_open_in_obsidian',{vaultId:id,path:note});
  if(!alive.current||ticket.current!==own||result.vaultId!==id||result.path!==note)return;
  setMessage(result.message);notify(result.message);
  setAccepted(result.status==='launchAccepted'?{binding:expected,path:note}:null);
  setConnection(current=>current?{...current,state:result.status==='launchFailed'?'launchFailed':current.state,lastRequest:result}:current);
 }
 async function run(action:()=>Promise<void>) {
  if(busy)return;setBusy(true);setError('');
  try{await action();}catch(e){if(alive.current)setError(aiError(e));}finally{if(alive.current)setBusy(false);}
 }
 function edit(){setAccepted(null);setMessage('');setError('');}
 async function configure() {
  const own=++ticket.current;
  const next=await invoke<KnowledgeSettings>('knowledge_configure_obsidian',{vaultId:id,kind,identifier:kind==='absolutePath'?null:identifier.trim(),registrationConfirmed:registered});
  if(!alive.current||ticket.current!==own||next.vaultId!==id)return;
  onSettings(next);setAccepted(null);setMessage('联动配置已保存；测试后请确认实际打开的笔记。');
  const status=await invoke<Connection>('knowledge_obsidian_connection',{vaultId:id});
  if(alive.current&&ticket.current===own&&status.vaultId===id)setConnection(status);
 }
 async function confirm() {
  if(!accepted)return;const own=++ticket.current;
  const next=await invoke<KnowledgeSettings>('knowledge_confirm_obsidian',{vaultId:id,expected:accepted.binding,path:accepted.path});
  if(!alive.current||ticket.current!==own||next.vaultId!==id)return;
  onSettings(next);setAccepted(null);setMessage('已记录你的确认：目标笔记已在 Obsidian 中打开。');
  setConnection(current=>current?{...current,binding:next.obsidian??null,state:'userConfirmed'}:current);
 }
 async function request(command:string,text:string,args:Record<string,unknown>={}) {
  await invoke(command,args);if(alive.current)setMessage(text);
 }

 return <LearningDialog title="连接 Obsidian" close={close} busy={busy}>
  <div className="obsidian-connection">
   <p className="obsidian-state" role="status">{connection?states[connection.state]:'正在检查 Obsidian URI…'}</p>
   <label>当前映流知识库<input readOnly value={vaultPath} onFocus={e=>e.currentTarget.select()}/></label>
   {path&&<p className="help">测试笔记：{path}</p>}
   <p className="help">当前笔记已保存在本地。联动失败时，可以继续在映流中编辑。</p>
   <ol><li>打开 Obsidian 的 Vault 管理器。</li><li>选择“将文件夹作为 Vault 打开”，选择上方的知识库根目录。</li><li>在 Vault 切换器右键复制 Vault ID，返回这里配置并测试。</li></ol>
   <div className="obsidian-actions">
    <button className="secondary" disabled={busy} onClick={()=>void run(()=>request('knowledge_obsidian_manager','已向 Obsidian 发送 Vault 管理器打开请求'))}>打开 Vault 管理器</button>
    <button className="secondary" disabled={busy} onClick={()=>void run(()=>request('knowledge_open_vault_folder','已请求打开知识库文件夹',{vaultId:id}))}>打开知识库文件夹</button>
    <button className="secondary" disabled={busy} onClick={()=>void run(async()=>{await navigator.clipboard.writeText(vaultPath);if(alive.current)setMessage('知识库路径已复制');})}>复制知识库路径</button>
   </div>
   <form onSubmit={e=>{e.preventDefault();void run(configure);}}>
    <label>Vault 标识方式<select value={kind} disabled={busy} onChange={e=>{setKind(e.target.value as ObsidianKind);edit();}}><option value="vaultId">Obsidian Vault ID（推荐）</option><option value="vaultName">明确指定 Vault 名称</option><option value="absolutePath">绝对路径兼容方式</option></select></label>
    {kind!=='absolutePath'&&<label>{kind==='vaultId'?'Obsidian Vault ID':'Obsidian Vault 名称'}<input required disabled={busy} maxLength={256} pattern={kind==='vaultId'?'[0-9a-fA-F]{16}':undefined} title={kind==='vaultId'?'从 Obsidian Vault 切换器复制 16 位 Vault ID':undefined} value={identifier} onChange={e=>{setIdentifier(e.target.value);edit();}}/></label>}
    {kind==='vaultName'&&<p className="help">请确认这是上方根目录对应的完整 Vault 名称，避免同名 Vault 打开错误笔记。</p>}
    {kind==='absolutePath'&&<p className="help">兼容方式仍要求此文件夹已在 Obsidian 中登记，不会自动注册 Vault。</p>}
    <label className="obsidian-check check-label"><input type="checkbox" disabled={busy} checked={registered} onChange={e=>{setRegistered(e.target.checked);edit();}}/><span>我已在 Obsidian 登记上方根目录，并确认{kind==='vaultName'?'所填 Vault 名称':kind==='vaultId'?'所填 Vault ID':'该目录'}对应此知识库</span></label>
    <button className="secondary" disabled={busy||!changed}>保存联动配置</button>
   </form>
   <div className="obsidian-actions"><button className="primary" disabled={!canTest} onClick={()=>{const expected=binding;if(expected&&path)void run(()=>launch(expected,path,++ticket.current));}}>测试打开当前笔记</button>{accepted&&<button className="primary" disabled={busy||changed} onClick={()=>void run(confirm)}>我已看到目标笔记，确认联动正常</button>}</div>
   {!path&&<p className="help">先在映流中选择一篇已保存的 Markdown 笔记，再测试打开。</p>}
   {changed&&<p className="help">请先保存当前联动配置，再测试打开。</p>}
   {message&&<p role="status" className="obsidian-message">{message}</p>}
   {error&&<p role="alert" className="error-text">{error}</p>}
   <details><summary>无法打开？</summary><p>Obsidian 可能尚未登记此知识库。请检查协议是否可用、确认 Vault 根目录和标识，再重新配置。系统接受 URI 请求并不能证明笔记已经打开。</p><p>未安装时，可从 Obsidian 官网下载安装并启动一次，以登记 URI 协议。映流知识库无需安装 Obsidian 即可使用。</p><button className="text-button" disabled={busy} onClick={()=>void run(()=>request('open_source','已请求打开 Obsidian 下载页面',{url:'https://obsidian.md/download'}))}>Obsidian 安装说明</button><button className="text-button" disabled={busy} onClick={()=>{edit();setRegistered(false);}}>重新配置</button></details>
   <footer><button className="secondary" disabled={busy} onClick={close}>稍后设置，继续在映流中编辑</button></footer>
  </div>
 </LearningDialog>;
}
