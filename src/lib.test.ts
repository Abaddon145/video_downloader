import test from 'node:test';
import assert from 'node:assert/strict';
import { extractUrls, pageItems, downloadError } from './lib.ts';
import { actionCount, batchConfirmation, batchSummary, escapeAction, keepSelection, selectRange, selectFiltered, taskActionAllowed } from './lib.ts';
import type { DownloadTask, TaskStatus } from './types.ts';
import { classifyCookieError, cookieExpiryText } from './lib.ts';
import { applyPreset } from './lib.ts';
import type { DownloadPreset } from './types.ts';
test('preset application copies all existing choices and naming without retaining mutable arrays', () => {
  const preset: DownloadPreset = {name:'音频',kind:'audio',videoMode:'source',maxHeight:720,subtitleLanguages:['en'],filenameTemplate:'{作者}_{标题}',byAuthor:true};
  const options=applyPreset(preset);preset.subtitleLanguages.push('zh-Hans');preset.filenameTemplate='{日期}';
  assert.equal(options.kind,'audio'); assert.equal(options.maxHeight,720); assert.deepEqual(options.subtitleLanguages,['en']);
  assert.equal(options.filenameTemplate,'{作者}_{标题}');assert.equal(options.byAuthor,true);
});

test('browser Cookie errors offer category-specific next steps without echoing secrets', () => {
  const cases = [
    ['ERROR: Failed to decrypt with DPAPI.', 'encrypted'],
    ['ERROR: could not find edge cookies database in profile', 'missing'],
    ['ERROR: Could not copy Chrome cookie database.', 'occupied'],
    ['ERROR: cookie database is locked', 'occupied'],
    ['ERROR: cookie reader failed', 'other'],
  ] as const;
  for (const [raw,category] of cases) { const result = classifyCookieError(raw); assert.equal(result?.category,category); assert.match(result!.message,/Cookie 文件/); }
  assert.equal(classifyCookieError('HTTP Error 412'),null);
  assert.equal(classifyCookieError('cookie reader failed Cookie: synthetic-tripwire')?.message.includes('synthetic-tripwire'),false);
  assert.match(downloadError('https://example.com','Failed to decrypt with DPAPI').message,/Cookie 文件/);
});
test('Cookie statistics distinguish expiry from session-only files', () => {
  assert.match(cookieExpiryText({count:2,sessionCount:0,earliestExpiry:1,latestExpiry:2},3),/Cookie 已过期，请重新导出/);
  assert.match(cookieExpiryText({count:2,sessionCount:2,earliestExpiry:null,latestExpiry:null},3),/会话.*有效期/);
  assert.doesNotMatch(cookieExpiryText({count:2,sessionCount:1,earliestExpiry:1,latestExpiry:2},3),/Cookie 已过期/);
});

const task = (id: string, status: TaskStatus): DownloadTask => ({ id, status } as DownloadTask);
test('selection survives progress events and removes only missing IDs', () => {
  assert.deepEqual([...keepSelection(new Set(['a','b']), [task('a','downloading'),task('b','paused')])], ['a','b']);
  assert.deepEqual([...keepSelection(new Set(['a','b']), [task('a','completed')])], ['a']);
});
test('Shift selects a visible range and select all respects the filter', () => {
  assert.deepEqual([...selectRange(new Set(['other']), ['a','b','c'], 'a','c',true)], ['other','a','b','c']);
  assert.deepEqual([...selectFiltered(new Set(['other']), ['a','b'],true)], ['other','a','b']);
  assert.deepEqual([...selectFiltered(new Set(['other','a']), ['a'],false)], ['other']);
});
test('toolbar counts follow single buttons and confirmation preserves files', () => {
  const items = [task('a','queued'),task('b','processing'),task('c','failed'),task('d','cancelled')];
  assert.equal(actionCount(items,'pause'),1); assert.equal(actionCount(items,'retry'),2);
  assert.equal(taskActionAllowed('failed','cancel'),false);
  assert.match(batchConfirmation('cancel',3), /3.*已下载的文件会保留/s);
  assert.match(batchConfirmation('remove',2), /2.*已下载的文件会保留/s);
  assert.equal(batchSummary({succeeded:['a'], skipped:[{id:'b',reason:'不可暂停'}],failed:[]}), '成功 1 项，跳过 1 项，失败 0 项');
});
test('Escape closes preview or cancels parsing before clearing selection', () => {
  assert.equal(escapeAction(true,false,2),'preview'); assert.equal(escapeAction(false,true,2),'preview');
  assert.equal(escapeAction(false,false,2),'selection'); assert.equal(escapeAction(false,false,0),'none');
});

test('extracts multiple share URLs without duplicates or surrounding Chinese punctuation', () => {
  assert.deepEqual(extractUrls('分享视频：https://www.bilibili.com/video/BV123。\nhttps://youtu.be/abc\nhttps://youtu.be/abc'), ['https://www.bilibili.com/video/BV123', 'https://youtu.be/abc']);
  assert.deepEqual(extractUrls('--exec calc\nfile:///secret'), []);
});
test('paginates 50 entries and safely clamps pages after deletion', () => {
  const items = Array.from({length: 105}, (_, i) => i);
  assert.deepEqual(pageItems(items, 3), [100,101,102,103,104]);
  assert.equal(pageItems(items, 1).length, 50);
  assert.deepEqual(pageItems([1, 2], 3), [1,2]);
});
test('explains Bilibili 412 without mistaking a bundled Python warning for the cause', () => {
  const raw = 'Deprecated Feature: Support for Python version 3.10 has been deprecated.\nERROR: [BiliBili] Unable to download webpage: HTTP Error 412: Precondition Failed';
  const error = downloadError('https://www.bilibili.com/video/BV17eHY6yEwc', raw);
  assert.match(error.title, /B站.*412/);
  assert.match(error.message, /浏览器.*Cookie/);
  assert.match(error.message, /无需.*Python/);
  assert.equal(downloadError('https://example.com/video', 'HTTP Error 412').message, 'HTTP Error 412');
});
