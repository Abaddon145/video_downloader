export function extractUrls(text: string): string[] {
  const matches = text.match(/https?:\/\/[^\s<>"'\u3000-\u303f\uff00-\uffef]+/gi) ?? [];
  return [...new Set(matches.map(url => url.replace(/[),.;]+$/, '')))];
}
export function pageItems<T>(items: T[], page: number): T[] {
  const validPage = Math.max(1, Math.min(page, Math.ceil(items.length / 50) || 1));
  return items.slice((validPage - 1) * 50, validPage * 50);
}
