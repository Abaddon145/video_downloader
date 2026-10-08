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
