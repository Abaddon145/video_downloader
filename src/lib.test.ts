import test from 'node:test';
import assert from 'node:assert/strict';
import { extractUrls, pageItems } from './lib.ts';

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
