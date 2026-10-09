import test from 'node:test';
import assert from 'node:assert/strict';
import { extractUrls, pageItems, downloadError } from './lib.ts';
import { actionCount, batchConfirmation, batchSummary, escapeAction, keepSelection, selectRange, selectFiltered, taskActionAllowed } from './lib.ts';
import type { DownloadTask, TaskStatus } from './types.ts';

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
