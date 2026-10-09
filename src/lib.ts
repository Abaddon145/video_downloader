import type { BatchAction, BatchResult, DownloadTask, TaskStatus } from './types.ts';
export function taskActionAllowed(status: TaskStatus, action: BatchAction): boolean {
  switch (action) {
    case 'pause': return ['queued','resolving','downloading'].includes(status);
    case 'resume': return status === 'paused';
    case 'cancel': return ['queued','resolving','downloading','processing','paused'].includes(status);
    case 'retry': return ['failed','cancelled'].includes(status);
    case 'pin': return ['queued','paused'].includes(status);
    case 'remove': return ['completed','failed','cancelled'].includes(status);
    case 'copy': return true;
  }
}
export function actionCount(tasks: DownloadTask[], action: BatchAction) { return tasks.filter(task => taskActionAllowed(task.status, action)).length; }
export function keepSelection(selected: Set<string>, tasks: DownloadTask[]) { const ids = new Set(tasks.map(task => task.id)); return new Set([...selected].filter(id => ids.has(id))); }
export function selectFiltered(selected: Set<string>, visible: string[], checked: boolean) { const next = new Set(selected); visible.forEach(id => checked ? next.add(id) : next.delete(id)); return next; }
export function selectRange(selected: Set<string>, visible: string[], anchor: string | null, id: string, checked: boolean) {
  const start = anchor == null ? -1 : visible.indexOf(anchor), end = visible.indexOf(id);
  return selectFiltered(selected, start < 0 || end < 0 ? [id] : visible.slice(Math.min(start,end),Math.max(start,end)+1),checked);
}
export function batchConfirmation(action: BatchAction, count: number) { return `确认${action === 'remove' ? '删除历史记录' : '取消任务'} ${count} 项？已下载的文件会保留。`; }
export function batchSummary(result: BatchResult) { return `成功 ${result.succeeded.length} 项，跳过 ${result.skipped.length} 项，失败 ${result.failed.length} 项`; }
export function escapeAction(preview: boolean, parsing: boolean, selected: number) { return preview || parsing ? 'preview' : selected ? 'selection' : 'none'; }

export function extractUrls(text: string): string[] {
  const matches = text.match(/https?:\/\/[^\s<>"'\u3000-\u303f\uff00-\uffef]+/gi) ?? [];
  return [...new Set(matches.map(url => url.replace(/[),.;]+$/, '')))];
}
export function pageItems<T>(items: T[], page: number): T[] {
  const validPage = Math.max(1, Math.min(page, Math.ceil(items.length / 50) || 1));
  return items.slice((validPage - 1) * 50, validPage * 50);
}
export function downloadError(url: string, raw: string): { title: string; message: string } {
  let host = '';
  try { host = new URL(url).hostname; } catch { /* Raw errors remain available for invalid links. */ }
  const bili = host === 'bilibili.com' || host.endsWith('.bilibili.com') || host === 'b23.tv' || /\[BiliBili[^\]]*\]/i.test(raw);
  if (bili && /(?:HTTP (?:Error )?412|blocked by server.*412)/i.test(raw)) {
    return { title: 'B站暂时拒绝了请求 · HTTP 412', message: '先在常用浏览器打开此视频，确认能够播放并完成页面要求的验证。然后到设置中读取该浏览器的 Cookie；读取失败时可导入 Cookie 文件。\n如已启用代理，请检查浏览器与软件的网络设置。调整后再解析，避免连续重试。登录状态不保证能解除源站限制。\nPython 版本提示来自捆绑内核，无需在电脑上安装或升级 Python。' };
  }
  return { title: '此链接处理失败', message: raw };
}
