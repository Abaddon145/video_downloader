import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { ArrowDownToLine, ArrowUpRight, Check, ChevronLeft, ChevronRight, CircleAlert, Clock3, Download, FileText, Folder, FolderOpen, Globe2, History, Link2, ListVideo, Loader2, Music2, Pause, Play, Plus, RefreshCw, Search, Settings2, ShieldCheck, SlidersHorizontal, Trash2, Video, Wrench, X, Zap, type LucideIcon } from 'lucide-react';
import { extractUrls, pageItems, downloadError, taskActionAllowed, actionCount, keepSelection, selectFiltered, selectRange, batchConfirmation, batchSummary, escapeAction, classifyCookieError, cookieExpiryText, applyPreset, latestCookieFailure } from './lib';
import type { BatchAction, BatchResult, DownloadPreset, PlaylistEntry } from './types';
import { emptyPreset, NamingFields, PresetSettings } from './Presets';
import MediaToolsPage from './pages/MediaToolsPage';
import type { AppSettings, AppSnapshot, DownloadRequest, DownloadTask, EngineUpdate, MediaKind, MediaPreview, PreviewResult, TaskStatus, VideoMode } from './types';

const initial: AppSnapshot = {
  settings: { downloadDir: '', concurrency: 2, cookieMode: 'none', browser: 'edge', browserProfile: '', hasCookieFile: false, proxyEnabled: false, proxyUrl: '', autoCheckCoreUpdate:true, lastCoreUpdateCheck:null, cookieSummary:null, downloadPresets:[] },
  tasks: [], engine: { version: '', ready: false, ffmpegVersion: '', denoVersion: '', error: null }, updating: false, previewing: false, notice: null, coreUpdate:null,
};
const statusText: Record<TaskStatus, string> = { queued: '等待中', resolving: '解析中', downloading: '下载中', processing: '处理中', paused: '已暂停', completed: '已完成', failed: '下载失败', cancelled: '已取消' };
const busyStatuses: TaskStatus[] = ['resolving', 'downloading', 'processing'];
const kindText = { video: '视频', audio: '音频', subtitles: '字幕' };
const languageName: Record<string, string> = { en: '英语', zh: '中文', 'zh-Hans': '简体中文', 'zh-Hant': '繁体中文', 'zh-CN': '简体中文', ja: '日语', ko: '韩语' };
const batchLabels: Record<BatchAction, string> = { pause:'暂停', resume:'恢复', cancel:'取消', retry:'重试', pin:'置顶', copy:'复制链接', remove:'删除历史' };
function errorText(error: unknown): string { return typeof error === 'string' ? error : error instanceof Error ? error.message : '操作未完成，请重试'; }
function bytes(value?: number | null) { if (value == null) return '未知'; const units = ['B', 'KB', 'MB', 'GB']; let unit = 0; while (value >= 1024 && unit < 3) { value /= 1024; unit++; } return `${value.toFixed(unit ? 1 : 0)} ${units[unit]}`; }
function duration(value?: number | null) { if (value == null) return '时长未知'; const seconds = Math.round(value); return seconds >= 3600 ? `${Math.floor(seconds / 3600)}:${String(Math.floor(seconds / 60) % 60).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}` : `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`; }
function siteName(url: string) { try { const host = new URL(url).hostname; return host.includes('bilibili') || host === 'b23.tv' ? '哔哩哔哩' : host.includes('youtu') ? 'YouTube' : host.replace(/^www\./, ''); } catch { return '视频来源'; } }
function dateText(ms: number) { return new Intl.DateTimeFormat('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }).format(ms); }

function IconButton({ icon: Icon, label, onClick, disabled = false, className = '' }: { icon: LucideIcon; label: string; onClick: () => void; disabled?: boolean; className?: string }) {
  return <button className={`icon-button ${className}`} title={label} aria-label={label} onClick={onClick} disabled={disabled}><Icon size={17} aria-hidden="true" /></button>;
}
function Cover({ src, kind = 'video', large = false }: { src: string | null; kind?: MediaKind; large?: boolean }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  const Icon = kind === 'audio' ? Music2 : kind === 'subtitles' ? FileText : Video;
  return <div className={`cover ${large ? 'large' : ''}`}>{src && !failed ? <img src={src} alt="视频封面" loading="lazy" referrerPolicy="no-referrer" onError={() => setFailed(true)} /> : <Icon size={large ? 36 : 22} aria-hidden="true" />}</div>;
}
function Pagination({ count, page, setPage }: { count: number; page: number; setPage: (page: number) => void }) {
  const pages = Math.max(1, Math.ceil(count / 50));
  const current = Math.min(page, pages);
  return <div className="pagination"><span>{count} 条记录 · 每页 50 条</span><div><IconButton icon={ChevronLeft} label="上一页" disabled={current <= 1} onClick={() => setPage(current - 1)} /><span>{current} / {pages}</span><IconButton icon={ChevronRight} label="下一页" disabled={current >= pages} onClick={() => setPage(current + 1)} /></div></div>;
}
function Modal({ title, subtitle, onClose, children, wide = false }: { title: string; subtitle?: string; onClose: () => void; children: ReactNode; wide?: boolean }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const close = useRef(onClose); close.current = onClose;
  useEffect(() => {
    const element = dialog.current!;
    const previous = document.activeElement as HTMLElement | null;
    element.showModal();
    return () => { element.close(); previous?.focus(); };
  }, []);
  return <dialog ref={dialog} className={`modal ${wide ? 'wide' : ''}`} aria-labelledby="modal-title" onKeyDown={event => {
    if (event.key !== 'Tab') return;
    // WebView2 lets native dialog Tab focus reach its host; keep the keyboard loop inside the dialog.
    const controls = [...event.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),summary,a[href],[tabindex="0"]')].filter(element => element.getClientRects().length > 0);
    const first = controls[0], last = controls.at(-1);
    if ((event.shiftKey && document.activeElement === first) || (!event.shiftKey && document.activeElement === last)) { event.preventDefault(); (event.shiftKey ? last : first)?.focus(); }
  }} onCancel={event => { event.preventDefault(); close.current(); }} onClick={event => { if (event.target === event.currentTarget) { const box = event.currentTarget.getBoundingClientRect(); if (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom) close.current(); } }}><div className="modal-header"><div><h2 id="modal-title">{title}</h2>{subtitle && <p>{subtitle}</p>}</div><IconButton icon={X} label="关闭对话框" onClick={onClose} /></div>{children}</dialog>;
}

function TaskRow({ task, act, details, checked, select }: { task: DownloadTask; act: (command: string, args: Record<string, unknown>, message?: string) => Promise<unknown>; details: () => void; checked: boolean; select: (checked: boolean, shift: boolean) => void }) {
  const active = busyStatuses.includes(task.status);
  const percent = task.progress.percent;
  const canPause = taskActionAllowed(task.status, 'pause');
  return <article className={`task-row ${task.status}`}>
    <input type="checkbox" aria-label={`选择任务：${task.request.title}`} aria-checked={checked} checked={checked} onChange={event => select(event.target.checked, (event.nativeEvent as MouseEvent).shiftKey)} />
    <Cover src={task.request.thumbnail} kind={task.request.kind} />
    <div className="task-main"><div className="task-title-line"><button className="task-title" onClick={details} title={task.request.title}>{task.request.title}</button><span className={`status-tag ${task.status}`}>{active && <span className="status-dot" />}{statusText[task.status]}</span></div>
      <div className="task-meta"><span>{siteName(task.request.url)}</span><span className="dot">·</span><span>{kindText[task.request.kind]}{task.request.kind === 'audio' ? ' / MP3' : task.request.kind === 'subtitles' ? ' / SRT' : task.request.videoMode === 'compatible' ? ' / MP4' : ' / 源格式'}</span><span className="dot">·</span><span>{task.request.maxHeight ? `最高 ${task.request.maxHeight}p` : '最佳可用画质'}</span>{task.status === 'completed' && <span className="task-date">{dateText(task.finishedAt ?? task.createdAt)}</span>}</div>
      {(active || task.status === 'paused') && <><div className={`progress-track ${percent == null && active ? 'indeterminate' : ''}`} role="progressbar" aria-label="下载进度" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent ?? undefined}><span style={{ width: `${percent ?? 25}%` }} /></div><div className="progress-caption"><span>{task.phase}</span><span>{percent != null ? `${percent.toFixed(1)}%` : '大小未知'}{task.progress.speed != null && task.status === 'downloading' ? ` · ${bytes(task.progress.speed)}/s` : ''}{task.progress.eta != null && task.status === 'downloading' ? ` · 剩余 ${duration(task.progress.eta)}` : ''}</span></div></>}
      {task.status === 'failed' && <button className="task-error" onClick={details}>{task.error ? downloadError(task.request.url, task.error).title : '下载失败，点击查看详情'}{task.error?.toLowerCase().includes('cookie') && ' · 可在设置中导入 Cookie 文件'}</button>}
    </div>
    <div className="task-actions">
      {canPause && <IconButton icon={Pause} label="暂停任务" onClick={() => { void act('control_task', { id: task.id, action: 'pause' }); }} />}
      {task.status === 'paused' && <IconButton icon={Play} label="恢复任务" onClick={() => { void act('control_task', { id: task.id, action: 'resume' }); }} />}
      {['failed', 'cancelled'].includes(task.status) && <IconButton icon={RefreshCw} label="重试任务" onClick={() => { void act('control_task', { id: task.id, action: 'retry' }); }} />}
      {task.status === 'completed' && <><IconButton icon={Play} label="打开文件" onClick={() => { void act('open_task_target', { id: task.id, folder: false }); }} /><IconButton icon={FolderOpen} label="打开所在目录" onClick={() => { void act('open_task_target', { id: task.id, folder: true }); }} /></>}
      {!['completed', 'cancelled', 'failed'].includes(task.status) && <IconButton icon={X} label="取消任务" onClick={() => { void act('control_task', { id: task.id, action: 'cancel' }); }} />}
      {['completed', 'cancelled', 'failed'].includes(task.status) && <IconButton icon={Trash2} label="移除记录，保留文件" onClick={() => { void act('remove_task', { id: task.id }, '记录已移除，下载文件保留在原目录'); }} />}
    </div>
  </article>;
}

function PreviewPanel({ results, expected, busy, directory, presets, savePreset, enqueue, close, configure, retry, openSource }: { presets: DownloadPreset[]; savePreset: (preset: DownloadPreset) => Promise<string|null>; results: PreviewResult[]; expected: number; busy: boolean; directory: string; enqueue: (requests: DownloadRequest[]) => Promise<string | null>; close: () => void; configure: () => void; retry: () => void; openSource: (url: string) => void }) {
  const [active, setActive] = useState(0);
  const [selected, setSelected] = useState(new Set<string>());
  const [kind, setKind] = useState<MediaKind>('video');
  const [mode, setMode] = useState<VideoMode>('compatible');
  const [height, setHeight] = useState(0);
  const [languages, setLanguages] = useState<string[]>([]);
  const [filenameTemplate, setFilenameTemplate] = useState('');
  const [byAuthor, setByAuthor] = useState(false);
  const [presetIndex, setPresetIndex] = useState('');
  const [presetName, setPresetName] = useState('');
  const [presetSaving, setPresetSaving] = useState(false);
  const [submissionError, setSubmissionError] = useState('');
  const [presetNotice,setPresetNotice]=useState('');
  const [playlistLanguages, setPlaylistLanguages] = useState('zh-Hans,en');
  const [page, setPage] = useState(1);
  const [saving, setSaving] = useState(false);
  const seen = useRef(new Set<string>());
  useEffect(() => {
    const added: string[] = [];
    for (const result of results) if (result.preview && !seen.current.has(result.url)) { seen.current.add(result.url); added.push(...(result.preview.isPlaylist ? result.preview.entries.slice(0, 1000).map(entry => entry.url) : [result.url])); }
    if (added.length) setSelected(previous => new Set([...previous, ...added]));
  }, [results]);
  const result = results[active];
  const preview = result?.preview;
  const failure = result?.error ? downloadError(result.url, result.error) : null;
  const allPreviews = results.flatMap(result => result.preview ? [result.preview] : []);
  const incompatible = kind === 'video' && mode === 'compatible' && allPreviews.some(item => !item.isPlaylist && selected.has(item.url) && !item.compatible);
  const allItems = allPreviews.flatMap<Omit<PlaylistEntry,'playlistIndex'> & {playlistIndex:number|null}>(item => item.isPlaylist ? item.entries : [{ id: item.url, url: item.url, title: item.title, thumbnail: item.thumbnail, duration: item.duration, playlistIndex: null }]);
  const chosen = allItems.filter((item, index) => selected.has(item.url) && allItems.findIndex(other => other.url === item.url) === index);
  const availableSubtitles = allPreviews.flatMap(item => item.subtitles).filter((track, index, all) => all.findIndex(other => other.language === track.language) === index);
  const hasPlaylist = allPreviews.some(item => item.isPlaylist);
  const subtitleLanguages = hasPlaylist ? playlistLanguages.split(',').map(lang => lang.trim()).filter(Boolean) : languages;
  const heights = [...new Set(preview?.formats.flatMap(format => format.height ? [format.height] : []) ?? [])].sort((a, b) => b - a);
  const toggle = (url: string) => setSelected(previous => { const next = new Set(previous); if (next.has(url)) next.delete(url); else next.add(url); return next; });
  const download = async () => {
    setSaving(true);
    try { setSubmissionError(await enqueue(chosen.map(item => ({ url: item.url, title: item.title, thumbnail: item.thumbnail, kind, videoMode: mode, maxHeight: height, subtitleLanguages: [...subtitleLanguages], filenameTemplate, byAuthor, playlistIndex: item.playlistIndex }))) || ''); } finally { setSaving(false); }
  };
  const choosePreset=(value:string)=>{setPresetIndex(value);const preset=applyPreset(value==='' ? emptyPreset : presets[Number(value)]);setKind(preset.kind);setMode(preset.videoMode);setHeight(preset.maxHeight);setLanguages(preset.subtitleLanguages);setPlaylistLanguages(value===''?'zh-Hans,en':preset.subtitleLanguages.join(','));setFilenameTemplate(preset.filenameTemplate);setByAuthor(preset.byAuthor);setSubmissionError('');};
  const storePreset=async()=>{setPresetSaving(true);try {const error=await savePreset({name:presetName.trim(),kind,videoMode:mode,maxHeight:height,subtitleLanguages:[...subtitleLanguages],filenameTemplate,byAuthor});setSubmissionError(error||'');if(!error){setPresetName('');setPresetNotice('预设已保存，可在设置中编辑。');}}finally{setPresetSaving(false);}};
  return <Modal title="解析与下载" subtitle={`${results.length} / ${expected} 个链接已解析${busy ? ' · 正在继续解析' : ''}`} onClose={close} wide>
    <div className="preview-body">
      {results.length > 1 && <div className="preview-tabs">{results.map((result, index) => <button key={result.url} className={active === index ? 'selected' : ''} onClick={() => { setActive(index); setPage(1); }}>{result.error ? <CircleAlert size={14} /> : <Video size={14} />}{result.preview?.title || siteName(result.url)}</button>)}</div>}
      {!results.length && <div className="preview-loading"><Loader2 className="spin" size={32} /><h3>正在读取视频信息</h3><p>封面、格式与字幕准备好后即可选择下载</p></div>}
      {result?.error && failure && <div className="error-panel"><CircleAlert size={24} /><h3>{failure.title}</h3><p>{failure.message}</p><div className="error-actions"><button className="secondary" onClick={() => openSource(result.url)}><ArrowUpRight size={16} />打开原网页</button><button className="secondary" disabled={busy} onClick={configure}><Settings2 size={16} />登录与网络设置</button><button className="secondary" disabled={busy} onClick={retry}><RefreshCw size={16} />重新解析</button></div>{failure.message !== result.error && <details className="task-logs"><summary>技术详情</summary><pre>{result.error}</pre></details>}</div>}
      {preview && <>
        <div className="preview-summary"><Cover src={preview.thumbnail} large /><div><div className="eyebrow">{siteName(preview.url)} {preview.isPlaylist && ' / 播放列表'}</div><h3>{preview.title}</h3><p>{preview.uploader || preview.site}{!preview.isPlaylist && ` · ${duration(preview.duration)}`}</p><div className="preview-facts"><span>{preview.isPlaylist ? `${preview.entries.length} 个视频` : `预计 ${bytes(preview.filesize)}`}</span>{!preview.isPlaylist && <span title="官方 MP4 预设当前选择的实际编码">{preview.videoCodec || '视频编码未知'} / {preview.audioCodec || '音频编码未知'}</span>}</div></div>{!preview.isPlaylist && <label className="check-label include"><input type="checkbox" checked={selected.has(preview.url)} onChange={() => toggle(preview.url)} />下载此视频</label>}</div>
        {preview.isPlaylist && <div className="playlist"><div className="playlist-heading"><label className="check-label"><input type="checkbox" checked={preview.entries.length > 0 && preview.entries.every(entry => selected.has(entry.url))} onChange={event => setSelected(previous => { const next = new Set(previous); preview.entries.forEach((entry, index) => { if (event.target.checked && index < 1000) next.add(entry.url); else next.delete(entry.url); }); return next; })} />选择列表{preview.entries.length > 1000 && '（每批最多 1000 条）'}</label><span>已选 {preview.entries.filter(entry => selected.has(entry.url)).length} 条</span></div><div className="playlist-items">{pageItems(preview.entries, page).map((entry, index) => <label className="playlist-item" key={`${entry.url}-${index}`}><input type="checkbox" checked={selected.has(entry.url)} onChange={() => toggle(entry.url)} /><span className="item-number">{(page - 1) * 50 + index + 1}</span><span className="item-title" title={entry.title}>{entry.title}</span><span>{duration(entry.duration)}</span></label>)}</div><Pagination count={preview.entries.length} page={page} setPage={setPage} /></div>}
      </>}
      {!!allPreviews.length && <div className="download-options"><div className="option-header"><SlidersHorizontal size={16} /><h3>下载选项</h3><span>应用到所有勾选条目</span></div>
        <label className="preset-picker">下载预设<select value={presetIndex} onChange={event=>choosePreset(event.target.value)}><option value="">不使用预设 · 默认选项</option>{presets.map((preset,index)=><option value={index} key={preset.name}>{preset.name}</option>)}</select></label>
        <div className="kind-picker">{([{ id: 'video', icon: Video, text: '视频', sub: '完整音视频' }, { id: 'audio', icon: Music2, text: '音频', sub: '转换为 MP3' }, { id: 'subtitles', icon: FileText, text: '字幕', sub: '单独保存 SRT' }] as const).map(({ id, icon: Icon, text, sub }) => <button key={id} className={kind === id ? 'selected' : ''} onClick={() => setKind(id)} aria-pressed={kind === id}><Icon size={20} /><span>{text}<small>{sub}</small></span>{kind === id && <Check size={15} />}</button>)}</div>
        {kind === 'video' && <div className="option-grid"><label>格式策略<select value={mode} onChange={event => setMode(event.target.value as VideoMode)}><option value="compatible">兼容优先 · MP4 / H.264 / AAC</option><option value="source">源站最高画质 · 保留源编码</option></select></label><label>最高画质<select value={height} onChange={event => setHeight(Number(event.target.value))}><option value={0}>最佳可用画质</option>{[...new Set([...heights, 2160, 1080, 720, 480, 360, height])].filter(Boolean).sort((a, b) => b - a).map(item => <option key={item} value={item}>{item}p</option>)}</select></label></div>}
        {incompatible && <div className="inline-warning"><CircleAlert size={16} />此视频没有 H.264 / AAC 组合，请选择“源站最高画质”。</div>}
        {(availableSubtitles.length > 0 || hasPlaylist || kind === 'subtitles') && <div className="subtitle-options"><label className="field-label">{kind === 'subtitles' ? '选择字幕语言' : '同时保存字幕 · SRT'}</label>{hasPlaylist ? <><input aria-label="字幕语言代码" value={playlistLanguages} onChange={event => setPlaylistLanguages(event.target.value)} placeholder="zh-Hans,en（以逗号分隔）" /><p className="help">播放列表逐条检查字幕；语言代码示例：zh-Hans、en。空白表示不附加字幕。</p></> : <div className="subtitle-tracks">{availableSubtitles.length ? availableSubtitles.map(track => <label className="check-label" key={track.language}><input type="checkbox" checked={languages.includes(track.language)} onChange={event => setLanguages(previous => event.target.checked ? [...previous, track.language] : previous.filter(lang => lang !== track.language))} />{languageName[track.language] || track.language}{track.automatic && <small>自动</small>}</label>) : <p className="help">此视频未提供可用字幕</p>}</div>}</div>}
        <NamingFields template={filenameTemplate} byAuthor={byAuthor} setTemplate={setFilenameTemplate} setByAuthor={setByAuthor} />
        <div className="preset-save-row"><label>保存当前选项为预设<input value={presetName} maxLength={40} placeholder="预设名称" onChange={event=>setPresetName(event.target.value)} /></label><button className="secondary" disabled={presetSaving || !presetName.trim() || presets.length>=20} onClick={()=>{void storePreset();}}>{presetSaving?'正在保存…':'保存预设'}（{presets.length}/20）</button></div>{presetNotice && <p className="help" role="status">{presetNotice}</p>}
        {submissionError && <p className="inline-warning" role="alert">{submissionError}</p>}
        <p className="help">文件大小为解析时的估算，实际格式会在下载前重新检查。</p>
      </div>}
    </div><div className="modal-footer"><div className="save-location" title={directory}><Folder size={16} /><span>{directory || '系统下载目录'}</span></div><button className="primary" disabled={busy || saving || !chosen.length || chosen.length > 1000 || incompatible || (kind === 'subtitles' && !subtitleLanguages.length)} onClick={() => { void download(); }}>{saving ? <Loader2 size={17} className="spin" /> : <ArrowDownToLine size={17} />}加入下载 · {chosen.length}</button></div>
  </Modal>;
}

function SettingsPanel({ snapshot, act, notify, cookieFailure }: { cookieFailure: ReturnType<typeof classifyCookieError>; snapshot: AppSnapshot; act: (command: string, args: Record<string, unknown>, message?: string) => Promise<unknown>; notify: (message: string) => void }) {
  const [form, setForm] = useState(snapshot.settings);
  const [proxy, setProxy] = useState('');
  const [proxyChanged, setProxyChanged] = useState(false);
  const [saving, setSaving] = useState(false);
  const [checking, setChecking] = useState(false);
  const [update, setUpdate] = useState<EngineUpdate | null>(null);
  const availableUpdate = update ?? snapshot.coreUpdate;
  const settingsKey = JSON.stringify({...snapshot.settings,lastCoreUpdateCheck:null,downloadPresets:[]});
  useEffect(() => { setForm(snapshot.settings); setProxy(''); setProxyChanged(false); }, [settingsKey]); // Backend snapshots may arrive while typing; only actual setting changes reset this form.
  const change = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => setForm(previous => ({ ...previous, [key]: value }));
  const chooseDirectory = async () => { try { const path = await open({ directory: true, multiple: false, title: '选择下载目录', defaultPath: form.downloadDir || undefined }); if (typeof path === 'string') change('downloadDir', path); } catch (error) { notify(errorText(error)); } };
  const importCookies = async () => { try { const path = await open({ multiple: false, title: '导入 Netscape 格式 Cookie', filters: [{ name: 'Cookie 文件', extensions: ['txt', 'cookies'] }] }); if (typeof path === 'string') await act('import_cookies', { path }, 'Cookie 已加密保存'); } catch (error) { notify(errorText(error)); } };
  const save = async () => { setSaving(true); await act('save_settings', { input: { downloadDir: form.downloadDir, concurrency: form.concurrency, cookieMode: form.cookieMode, browser: form.browser, browserProfile: form.browserProfile, proxyEnabled: form.proxyEnabled, proxyUrl: proxyChanged ? proxy : null, autoCheckCoreUpdate: form.autoCheckCoreUpdate } }, '设置已保存，将应用于新任务'); setSaving(false); };
  const checkUpdate = async () => { setChecking(true); try { const result = await invoke<EngineUpdate>('check_engine_update'); setUpdate(result); if (!result.available) notify('当前已是官方 stable 最新版本'); } catch (error) { notify(errorText(error)); } finally { setChecking(false); } };
  const applyUpdate = async () => { const result = await act('update_engine', {}, '内核更新完成'); if (result) setUpdate(null); };
  const busy = snapshot.previewing || snapshot.tasks.some(task => ['queued', ...busyStatuses].includes(task.status));
  return <div className="settings-content">
    <PresetSettings presets={snapshot.settings.downloadPresets ?? []} save={async presets=>!!await act('save_download_presets',{presets},'下载预设已保存')} />
    <section className="settings-card"><div className="section-icon"><Folder size={20} /></div><div className="settings-section"><h2>下载与保存</h2><p className="section-description">配置新任务的默认保存位置与同时下载数量</p><label className="field-label" htmlFor="download-directory">保存目录</label><div className="input-with-button"><input id="download-directory" value={form.downloadDir} onChange={event => change('downloadDir', event.target.value)} /><button className="secondary" onClick={() => { void chooseDirectory(); }}><FolderOpen size={16} />浏览</button></div><div className="setting-line"><div><label htmlFor="concurrency">同时下载数量</label><p className="help">已运行任务会先完成；新任务按此数量排队</p></div><select id="concurrency" value={form.concurrency} onChange={event => change('concurrency', Number(event.target.value))}>{[1, 2, 3, 4].map(value => <option value={value} key={value}>{value} 个任务{value === 2 ? ' · 推荐' : ''}</option>)}</select></div></div></section>
    <section className="settings-card"><div className="section-icon"><ShieldCheck size={20} /></div><div className="settings-section"><h2>登录与 Cookie</h2><p className="section-description">下载需要登录的内容时，使用你的本地登录状态</p><div className="radio-group">{[{ id: 'none', text: '不使用 Cookie' }, { id: 'browser', text: '从浏览器读取' }, { id: 'file', text: '导入 Cookie 文件' }].map(item => <label key={item.id}><input type="radio" name="cookie-mode" value={item.id} checked={form.cookieMode === item.id} onChange={() => change('cookieMode', item.id)} />{item.text}</label>)}</div>
      {form.cookieMode === 'browser' && <><div className="option-grid"><label>浏览器<select value={form.browser} onChange={event => change('browser', event.target.value)}><option value="edge">Microsoft Edge</option><option value="chrome">Google Chrome</option><option value="firefox">Mozilla Firefox</option></select></label><label>配置名称 / 路径（可选）<input value={form.browserProfile} onChange={event => change('browserProfile', event.target.value)} placeholder="使用默认配置" /></label></div><div className="inline-note"><CircleAlert size={16} />读取可能受浏览器占用或加密限制。失败时请切换为 Cookie 文件导入。</div></>}
      {cookieFailure && <div className="inline-warning cookie-failure" role="status"><CircleAlert size={16} /><div><strong>{cookieFailure.title}</strong><p>{cookieFailure.message}</p></div></div>}
      {form.cookieMode === 'file' && <div className="cookie-import"><div><strong>{snapshot.settings.hasCookieFile ? '已导入 Cookie 文件' : '尚未导入 Cookie 文件'}</strong><p className="help">Netscape 格式 · 由 Windows 加密保存在本机</p></div><button className="secondary" onClick={() => { void importCookies(); }}><Plus size={16} />{snapshot.settings.hasCookieFile ? '重新导入' : '选择文件'}</button></div>}
      {form.cookieMode === 'file' && snapshot.settings.hasCookieFile && <p className="cookie-statistics" role="status">{snapshot.settings.cookieSummary ? cookieExpiryText(snapshot.settings.cookieSummary) : '有效期信息不可用，重新导入后可查看统计。'}</p>}
      <details className="cookie-guide"><summary>如何导出 Cookie 文件</summary><ol><li>在常用浏览器中登录自己的账号，打开来源视频，确认可以播放。</li><li>使用浏览器提供或你信任的导出方式，将该站点的 Cookie 保存为 Netscape HTTP Cookie File；部分浏览器没有直接导出选项。</li><li>文件需使用 UTF-8 编码、小于 2 MiB，每条记录为制表符分隔的 7 列。不要直接复制 JSON 或 HTTP 请求头。</li><li>选择“导入 Cookie 文件”，导入后检查有效期并保存设置；过期时重新导出。</li></ol><p>Cookie 等同登录凭据，不要分享，只导入自己的账号。软件仅显示数量与有效期统计。</p></details>
    </div></section>
    <section className="settings-card"><div className="section-icon"><Globe2 size={20} /></div><div className="settings-section"><div className="setting-line top"><div><h2>网络代理</h2><p className="section-description">为解析、下载和内核更新指定代理</p></div><label className="switch"><input type="checkbox" aria-label="启用网络代理" checked={form.proxyEnabled} onChange={event => change('proxyEnabled', event.target.checked)} /><span /></label></div><label className="field-label" htmlFor="proxy-url">代理地址</label><input id="proxy-url" value={proxy} onChange={event => { setProxy(event.target.value); setProxyChanged(true); }} placeholder={snapshot.settings.proxyUrl ? `已保存：${snapshot.settings.proxyUrl}（留空保留）` : 'http://127.0.0.1:7890'} autoComplete="off" spellCheck={false} /><p className="help">支持 HTTP、HTTPS、SOCKS5。含账号密码的地址会加密保存。</p></div></section>
    <div className="settings-save"><span>关闭窗口后，下载继续在托盘运行</span><button className="primary" disabled={saving} onClick={() => { void save(); }}>{saving ? <Loader2 className="spin" size={16} /> : <Check size={16} />}保存设置</button></div>
    <section className="settings-card engine-card"><div className="section-icon"><Zap size={20} /></div><div className="settings-section"><div className="setting-line top"><div><h2>下载内核</h2><p className="section-description">yt-dlp {snapshot.engine.version || '正在检查'} <span className="small-tag">stable</span></p></div><button className="secondary" disabled={checking || snapshot.updating || !snapshot.engine.ready} onClick={() => { void checkUpdate(); }}><RefreshCw size={15} className={checking ? 'spin' : ''} />{checking ? '正在检查' : '检查更新'}</button></div>{availableUpdate?.available && <div className="update-available"><span>新版本 {availableUpdate.version} 已发布</span><button className="primary" disabled={busy || snapshot.updating} onClick={() => { void applyUpdate(); }}>{snapshot.updating ? '正在校验并更新…' : '更新内核'}</button></div>}{availableUpdate?.available && busy && <p className="help">请先暂停或完成所有解析与下载，再更新内核。</p>}{snapshot.engine.error && <p className="error-text">{snapshot.engine.error}</p>}<div className="setting-line"><div><label htmlFor="auto-core-update">启动后检查内核更新</label><p className="help">启动约 30 秒后静默检查，每 24 小时最多一次；有新版本时仅提示。</p>{snapshot.settings.lastCoreUpdateCheck && <p className="help">上次自动检查：{dateText(snapshot.settings.lastCoreUpdateCheck)}</p>}</div><label className="switch"><input id="auto-core-update" type="checkbox" checked={form.autoCheckCoreUpdate} onChange={event => change('autoCheckCoreUpdate',event.target.checked)} /><span /></label></div><div className="engine-dependencies"><span><Check size={14} />FFmpeg / ffprobe</span><span><Check size={14} />{snapshot.engine.denoVersion || 'Deno'}</span><span><ShieldCheck size={14} />SHA256 校验与失败回滚</span></div><details className="component-details"><summary>组件版本与来源</summary><p>{snapshot.engine.ffmpegVersion || 'FFmpeg 尚未就绪'}</p><p>{snapshot.engine.denoVersion || 'Deno 尚未就绪'}</p><button className="text-button" onClick={() => { void act('open_source', { url: 'https://github.com/yt-dlp/yt-dlp' }); }}>yt-dlp 官方项目 <ArrowUpRight size={14} /></button><p>组件许可与校验清单包含在安装目录中。</p></details></div></section>
  </div>;
}

export default function App() {
  const [snapshot, setSnapshot] = useState(initial);
  const [page, setPage] = useState<'download' | 'media' | 'history' | 'settings'>('download');
  const [links, setLinks] = useState('');
  const [filter, setFilter] = useState('all');
  const [search, setSearch] = useState('');
  const [listPage, setListPage] = useState(1);
  const [toast, setToast] = useState('');
  const [previewOpen, setPreviewOpen] = useState(false);
  const [results, setResults] = useState<PreviewResult[]>([]);
  const [expected, setExpected] = useState(0);
  const [parsing, setParsing] = useState(false);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [cookieFailure, setCookieFailure] = useState<ReturnType<typeof classifyCookieError>>(null);
  const [selectedIds, setSelectedIds] = useState(new Set<string>());
  const [confirmBatch, setConfirmBatch] = useState<{action: BatchAction; ids: string[]} | null>(null);
  const [batchResult, setBatchResult] = useState<BatchResult | null>(null);
  const [batchBusy, setBatchBusy] = useState(false);
  const selectionAnchor = useRef<string | null>(null);
  const input = useRef<HTMLTextAreaElement>(null);
  const previewButton = useRef<HTMLButtonElement>(null);
  const previewVisible = useRef(false); previewVisible.current = previewOpen;
  const lastSnapshot = useRef<AppSnapshot|null>(null);
  const cookieFailureTask = useRef<string|null>(null);
  const notify = useCallback((message: string) => setToast(message), []);
  const receiveSnapshot = useCallback((data: AppSnapshot) => {
    const previous=lastSnapshot.current;
    lastSnapshot.current=data;
    setSnapshot(data);
    const failed = latestCookieFailure(data.tasks,previous?.tasks ?? (data.settings.cookieMode==='file' && data.settings.hasCookieFile ? data.tasks : []));
    if (failed?.error) { cookieFailureTask.current=failed.id;setCookieFailure(classifyCookieError(failed.error)); }
    else if (cookieFailureTask.current && !data.tasks.some(task=>task.id===cookieFailureTask.current && task.status==='failed' && !!task.error && !!classifyCookieError(task.error))) { cookieFailureTask.current=null;setCookieFailure(null); }
  }, []);
  const act = useCallback(async (command: string, args: Record<string, unknown>, message?: string): Promise<unknown> => {
    try { if (!isTauri()) throw new Error('请在 Windows 桌面软件中使用此功能'); const result = await invoke(command, args); if(command==='import_cookies') {cookieFailureTask.current=null;setCookieFailure(null);} if (message) notify(message); return result ?? true; } catch (error) { notify(errorText(error)); return null; }
  }, [notify]);
  useEffect(() => {
    if (!isTauri()) { setSnapshot(previous => ({ ...previous, notice: '当前为界面预览。下载功能在 Windows 桌面软件中运行。' })); return; }
    let cancelled = false;
    const stops: (() => void)[] = [];
    const subscribe = async () => {
      for (const [event, handler] of [
        ['snapshot-updated', receiveSnapshot],
        ['task-updated', (task: DownloadTask) => { const previous=lastSnapshot.current??initial;receiveSnapshot({...previous,tasks:previous.tasks.some(item=>item.id===task.id) ? previous.tasks.map(item=>item.id===task.id ? task : item) : [...previous.tasks,task]}); }],
        ['preview-result', (result: PreviewResult) => { cookieFailureTask.current=null;setCookieFailure(result.error ? classifyCookieError(result.error) : null); if (previewVisible.current) setResults(previous => [...previous.filter(item => item.url !== result.url), result]); }],
      ] as const) {
        const stop = await listen(event, event => { if (!cancelled) handler(event.payload as never); });
        if (cancelled) stop(); else stops.push(stop);
      }
      const state = await invoke<AppSnapshot>('get_snapshot');
      if (!cancelled) receiveSnapshot(state);
    };
    void subscribe().catch(error => notify(errorText(error)));
    return () => { cancelled = true; stops.forEach(stop => stop()); };
  }, [notify, receiveSnapshot]);
  useEffect(() => { if (!toast) return; const timer = setTimeout(() => setToast(''), 7000); return () => clearTimeout(timer); }, [toast]);
  useEffect(() => { const key = (event: KeyboardEvent) => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') { event.preventDefault(); setPage('download'); requestAnimationFrame(() => input.current?.focus()); } }; document.addEventListener('keydown', key); return () => document.removeEventListener('keydown', key); }, []);
  useEffect(() => setListPage(1), [page, filter, search]);
  useEffect(() => { setSelectedIds(previous => keepSelection(previous, snapshot.tasks)); }, [snapshot.tasks]);
  const urls = extractUrls(links);
  const active = snapshot.tasks.filter(task => busyStatuses.includes(task.status));
  const waiting = snapshot.tasks.filter(task => task.status === 'queued');
  const completed = snapshot.tasks.filter(task => task.status === 'completed');
  const speed = active.reduce((sum, task) => sum + (task.status === 'downloading' ? task.progress.speed ?? 0 : 0), 0);
  const history = snapshot.tasks.filter(task => ['completed', 'failed', 'cancelled'].includes(task.status));
  const list = (page === 'history' ? history : snapshot.tasks).filter(task => (filter === 'all' || (filter === 'active' ? busyStatuses.includes(task.status) : task.status === filter)) && `${task.request.title} ${task.request.url}`.toLowerCase().includes(search.toLowerCase())).sort((a, b) => page === 'history' ? (b.finishedAt ?? b.createdAt) - (a.finishedAt ?? a.createdAt) : (b.queueOrder - a.queueOrder) || a.createdAt - b.createdAt);
  const selectedTasks = snapshot.tasks.filter(task => selectedIds.has(task.id));
  const filteredIds = list.map(task => task.id);
  const visibleIds = pageItems(list, listPage).map(task => task.id);
  const detail = snapshot.tasks.find(task => task.id === detailId);
  const startPreview = async () => {
    if (!urls.length) { notify('请粘贴完整的视频或播放列表链接'); input.current?.focus(); return; }
    if (urls.length > 100) { notify('每次最多解析 100 个链接'); return; }
    setExpected(urls.length); setResults([]); setPreviewOpen(true); setParsing(true);
    try { const data = await invoke<PreviewResult[]>('preview_sources', { urls }); if (previewVisible.current) setResults(data); } catch (error) { notify(errorText(error)); if (previewVisible.current) setResults([{ url: urls[0], preview: null, error: errorText(error) }]); } finally { setParsing(false); }
  };
  const closePreview = () => { if (parsing) void act('cancel_preview', {}); setPreviewOpen(false); requestAnimationFrame(() => (parsing ? input.current : previewButton.current)?.focus()); };
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || (!previewOpen && (event.target as HTMLElement)?.closest('dialog'))) return;
      const action = escapeAction(previewOpen, parsing || snapshot.previewing, selectedIds.size);
      if (action === 'preview') { event.preventDefault(); if (parsing || snapshot.previewing) void act('cancel_preview', {}); setPreviewOpen(false); }
      else if (action === 'selection') { event.preventDefault(); setSelectedIds(new Set()); }
    };
    document.addEventListener('keydown', key); return () => document.removeEventListener('keydown', key);
  }, [previewOpen, parsing, snapshot.previewing, selectedIds.size, act]);
  const runBatch = async (action: BatchAction, ids: string[]) => {
    setConfirmBatch(null); setBatchBusy(true);
    try {
      const result = await act('batch_task_action', {ids,action}) as BatchResult | null;
      if (result) {
        if (action === 'copy') {
          try { await navigator.clipboard.writeText(result.succeeded.map(id => snapshot.tasks.find(task => task.id === id)?.request.url).filter(Boolean).join('\n')); }
          catch { result.failed.push(...result.succeeded.map(id => ({id,error:'剪贴板写入失败，请重试'}))); result.succeeded = []; }
        }
        setBatchResult(result);
      }
    } finally { setBatchBusy(false); }
  };
  const requestBatch = (action: BatchAction) => { const ids = [...selectedIds]; if (action === 'cancel' || action === 'remove') setConfirmBatch({action, ids}); else void runBatch(action,ids); };
  const enqueue = async (requests: DownloadRequest[]) => {
    try { if (!isTauri()) throw new Error('请在 Windows 桌面软件中使用此功能'); const tasks = await invoke<DownloadTask[]>('enqueue_downloads', {requests});
    if (tasks) { setPreviewOpen(false); setLinks(''); notify(tasks.length ? `${tasks.length} 个任务已加入下载队列` : '所选任务已在队列中'); setFilter('all'); setPage('download'); requestAnimationFrame(() => input.current?.focus()); }
    return null; } catch(error) { return errorText(error); }
  };
  const savePreviewPreset=async(preset:DownloadPreset)=>{try {await invoke('save_download_presets',{presets:[...snapshot.settings.downloadPresets,preset]});return null;}catch(error){return errorText(error);}};
  const navigate = (target: typeof page) => { setPage(target); setFilter('all'); setSearch(''); };
  return <div className="app-shell">
    <aside className="sidebar"><div className="brand"><div className="brand-mark"><ArrowDownToLine size={23} strokeWidth={2.5} /></div><div><strong>映流</strong><span>视频下载器</span></div></div><div className="nav-label">工作空间</div><nav aria-label="主导航">{[{ id: 'download', icon: Download, text: '下载', badge: active.length + waiting.length }, { id: 'media', icon: Wrench, text: '媒体工具', badge: null }, { id: 'history', icon: History, text: '历史', badge: null }, { id: 'settings', icon: Settings2, text: '设置', badge: null }].map(({ id, icon: Icon, text, badge }) => <button key={id} className={page === id ? 'active' : ''} onClick={() => navigate(id as typeof page)} aria-current={page === id ? 'page' : undefined}><Icon size={19} /><span>{text}</span>{!!badge && <span className="nav-badge">{badge}</span>}</button>)}</nav>
      <div className="sidebar-bottom"><div className="local-card"><ShieldCheck size={18} /><div><strong>本地运行</strong><p>下载文件只保存在你的电脑</p></div></div><div className="engine-status"><span className={`status-dot ${snapshot.engine.ready ? 'online' : ''}`} /><span>{snapshot.engine.ready ? `yt-dlp ${snapshot.engine.version}` : snapshot.engine.error ? '组件检查失败' : '正在检查组件'}</span></div><div className="app-version">WINDOWS DESKTOP <span>v0.1.1</span></div></div>
    </aside>
    <main className="main"><header className="page-header"><div><div className="eyebrow">你的本地媒体工作空间</div><h1>{page === 'download' ? '下载' : page === 'history' ? '下载历史' : page === 'media' ? '媒体工具' : '偏好设置'}</h1><p>{page === 'download' ? '把喜欢的内容，留在本地。' : page === 'history' ? '每一次保存，都有迹可循。' : page === 'media' ? '转换、裁剪与提取，让媒体更合用。' : '让下载按照你的习惯运行。'}</p></div><div className="header-right">{snapshot.coreUpdate?.available && <button className="text-button" onClick={() => navigate('settings')}>内核有新版本 {snapshot.coreUpdate.version}，去更新</button>}<span className="local-badge"><span className="status-dot online" />本地存储</span>{page === 'history' && <button className="secondary" onClick={() => { navigate('download'); requestAnimationFrame(() => input.current?.focus()); }}><Plus size={16} />新建下载</button>}</div></header>
      {(snapshot.notice || snapshot.engine.error) && <div className="global-notice" role="status"><CircleAlert size={17} /><span>{snapshot.notice || snapshot.engine.error}</span></div>}
      {page === 'media' ? <MediaToolsPage notify={notify} /> : page === 'settings' ? <SettingsPanel snapshot={snapshot} act={act} notify={notify} cookieFailure={cookieFailure} /> : <>
        {page === 'download' && <><section className="link-card"><div className="link-card-heading"><span><Link2 size={18} />添加视频链接</span><kbd>Ctrl + L</kbd></div><div className="link-input-row"><textarea ref={input} aria-label="视频链接" value={links} onChange={event => setLinks(event.target.value)} onKeyDown={event => { if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') { event.preventDefault(); if (snapshot.engine.ready && !parsing) void startPreview(); } }} placeholder="粘贴视频、播放列表或分享文本，可一次添加多个链接" rows={2} spellCheck={false} /><button ref={previewButton} className="primary parse-button" disabled={!urls.length || parsing || snapshot.previewing || snapshot.updating || !snapshot.engine.ready} onClick={() => { void startPreview(); }}>{parsing ? <Loader2 size={18} className="spin" /> : <Search size={18} />}解析链接</button></div><div className="link-hint"><span><Globe2 size={14} />支持哔哩哔哩、YouTube 及 yt-dlp 支持的平台</span><span>{urls.length ? `识别到 ${urls.length} 个链接` : '先预览，再下载'}</span></div></section>
        <div className="stats-strip"><div><span className="stat-icon cyan"><Download size={19} /></span><div><span>正在下载</span><strong>{active.length}<small> / {snapshot.settings.concurrency}</small></strong></div></div><div><span className="stat-icon"><Clock3 size={19} /></span><div><span>等待下载</span><strong>{waiting.length}<small> 个任务</small></strong></div></div><div><span className="stat-icon green"><Check size={19} /></span><div><span>已保存</span><strong>{completed.length}<small> 个文件任务</small></strong></div></div><div><span className="stat-icon"><Zap size={19} /></span><div><span>当前速度</span><strong className="speed-stat">{speed ? bytes(speed) : '—'}<small>{speed ? '/s' : ''}</small></strong></div></div></div></>}
        <section className="task-list" tabIndex={0} aria-label="下载任务列表" onKeyDown={event => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a' && !(event.target as HTMLElement).matches('input:not([type=checkbox]),textarea,select')) { event.preventDefault(); setSelectedIds(previous => selectFiltered(previous,filteredIds,true)); } }}><div className="list-header"><div className="list-title"><h2>{page === 'history' ? '全部记录' : '下载队列'}</h2><span>{page === 'history' ? history.length : snapshot.tasks.length}</span></div><div className="list-tools">{page === 'history' && <div className="search-field"><Search size={15} /><input aria-label="搜索下载历史" placeholder="搜索标题或链接" value={search} onChange={event => setSearch(event.target.value)} /></div>}<select aria-label="筛选任务状态" value={filter} onChange={event => setFilter(event.target.value)}><option value="all">全部状态</option>{(page === 'history' ? ['completed', 'failed', 'cancelled'] : ['queued', 'active', 'paused', 'failed', 'completed', 'cancelled']).map(value => <option key={value} value={value}>{value === 'active' ? '下载中（含解析与处理）' : statusText[value as TaskStatus]}</option>)}</select></div></div>
          <div className="selection-line"><label className="check-label"><input type="checkbox" aria-label="全选当前筛选结果" aria-checked={!!list.length && filteredIds.every(id => selectedIds.has(id))} checked={!!list.length && filteredIds.every(id => selectedIds.has(id))} onChange={event => setSelectedIds(previous => selectFiltered(previous,filteredIds,event.target.checked))} />全选当前筛选结果</label><span>已选 {selectedIds.size} 项，当前筛选共 {list.length} 项</span></div>
          {!!selectedIds.size && <div className="batch-toolbar" role="group" aria-label="批量任务操作">{(Object.keys(batchLabels) as BatchAction[]).map(action => <button key={action} className="secondary" disabled={batchBusy || actionCount(selectedTasks,action) === 0} onClick={() => requestBatch(action)}>{batchLabels[action]}（{actionCount(selectedTasks,action)}）</button>)}<button className="text-button" disabled={batchBusy} onClick={() => setSelectedIds(new Set())}>清除选择</button></div>}
          {list.length ? <div className="task-items">{pageItems(list, listPage).map(task => <TaskRow key={task.id} task={task} act={act} details={() => setDetailId(task.id)} checked={selectedIds.has(task.id)} select={(checked, shift) => { const anchor = selectionAnchor.current; setSelectedIds(previous => shift ? selectRange(previous,visibleIds,anchor,task.id,checked) : selectFiltered(previous,[task.id],checked)); selectionAnchor.current = task.id; }} />)}</div> : <div className="empty-state"><div className="empty-art"><div className="empty-line line-one" /><div className="empty-line line-two" /><div className="empty-sheet"><span /><span /><ArrowDownToLine size={28} /></div><span className="empty-spark"><Plus size={15} /></span></div><h3>{page === 'history' ? '下载记录会保存在这里' : filter === 'all' ? '准备好保存下一个好视频' : '没有此状态的任务'}</h3><p>{page === 'history' ? '完成、失败和取消的任务，都可以在这里查看。' : filter === 'all' ? '粘贴链接，选择画质，剩下的交给映流。' : '切换筛选条件，或添加一个新链接。'}</p>{page === 'download' && filter === 'all' && <button className="text-button" onClick={() => input.current?.focus()}>添加第一个链接 <ArrowUpRight size={15} /></button>}</div>}
          {!!list.length && <Pagination count={list.length} page={listPage} setPage={setListPage} />}
        </section>
        {page === 'download' && <div className="workspace-footer"><span className="save-location" title={snapshot.settings.downloadDir}><Folder size={15} /><span>{snapshot.settings.downloadDir || '默认保存至「下载 / 视频下载」'}</span></span><button className="text-button" onClick={() => navigate('settings')}>更改位置 <ChevronRight size={14} /></button></div>}
      </>}
    </main>
    {!!toast && <div className="toast" role="status"><CircleAlert size={18} /><span>{toast}</span><IconButton icon={X} label="关闭提示" onClick={() => setToast('')} /></div>}
    {confirmBatch && <Modal title={`确认${batchLabels[confirmBatch.action]}`} onClose={() => setConfirmBatch(null)}><div className="details-body"><p>{batchConfirmation(confirmBatch.action,confirmBatch.ids.length)}</p><p className="help">不适用的任务会跳过并说明原因。</p></div><div className="modal-footer"><button className="secondary" onClick={() => setConfirmBatch(null)}>返回</button><button className="primary" onClick={() => { void runBatch(confirmBatch.action,confirmBatch.ids); }}>确认操作</button></div></Modal>}
    {batchResult && <Modal title="批量操作结果" onClose={() => setBatchResult(null)}><div className="details-body"><p role="status">{batchSummary(batchResult)}</p>{!!(batchResult.skipped.length + batchResult.failed.length) && <details className="batch-details"><summary>查看逐项原因</summary>{batchResult.skipped.map(item => <p key={item.id}>{snapshot.tasks.find(task => task.id === item.id)?.request.title || item.id}：{item.reason}</p>)}{batchResult.failed.map(item => <p className="error-text" key={item.id}>{snapshot.tasks.find(task => task.id === item.id)?.request.title || item.id}：{item.error}</p>)}</details>}</div><div className="modal-footer"><button className="primary" onClick={() => setBatchResult(null)}>知道了</button></div></Modal>}
    {previewOpen && <PreviewPanel results={results} expected={expected} busy={parsing} directory={snapshot.settings.downloadDir} presets={snapshot.settings.downloadPresets ?? []} savePreset={savePreviewPreset} enqueue={enqueue} close={closePreview} configure={() => { closePreview(); navigate('settings'); }} retry={() => { void startPreview(); }} openSource={url => { void act('open_source', { url }); }} />}
    {detail && <Modal title="任务详情" subtitle={statusText[detail.status]} onClose={() => setDetailId(null)}><div className="details-body"><h3>{detail.request.title}</h3><dl><dt>来源链接</dt><dd>{detail.request.url}</dd><dt>保存目录</dt><dd>{detail.outputDir}</dd><dt>下载类型</dt><dd>{kindText[detail.request.kind]} · {detail.request.videoMode === 'compatible' ? '兼容优先' : '源格式'}</dd><dt>创建时间</dt><dd>{dateText(detail.createdAt)}</dd>{detail.files.length > 0 && <><dt>输出文件</dt><dd>{detail.files.map(file => <p key={file}>{file}</p>)}</dd></>}</dl>{detail.error && <div className="error-panel"><strong>{downloadError(detail.request.url, detail.error).title}</strong><p>{downloadError(detail.request.url, detail.error).message}</p><details className="task-logs"><summary>技术详情</summary><pre>{detail.error}</pre></details>{(detail.error.toLowerCase().includes('cookie') || downloadError(detail.request.url, detail.error).message !== detail.error) && <button className="secondary" onClick={() => { setDetailId(null); navigate('settings'); }}>到设置导入 Cookie</button>}</div>}<details className="task-logs" open={detail.status === 'failed'}><summary>内核日志（已脱敏）</summary><pre>{detail.logs.join('\n') || '暂无日志'}</pre></details></div><div className="modal-footer"><button className="secondary" onClick={() => { void act('open_source', { url: detail.request.url }); }}><ArrowUpRight size={16} />打开来源</button><button className="secondary" onClick={() => { void act('open_task_target', { id: detail.id, folder: true }); }}><FolderOpen size={16} />打开目录</button></div></Modal>}
  </div>;
}
