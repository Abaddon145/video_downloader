import test from 'node:test';
import assert from 'node:assert/strict';
const resize=await import('./resize.ts').catch(()=>({}));
test('resize clamps pointer movement and keyboard boundaries',()=>{
 assert.equal(typeof resize.resizeValue,'function','resize logic is available');
 assert.equal(resize.resizeValue(360,900,240,640),640);
 assert.equal(resize.resizeValue(360,-900,240,640),240);
 assert.equal(resize.resizeKey(360,'ArrowLeft',240,640),350);
 assert.equal(resize.resizeKey(360,'ArrowLeft',240,640,10,true),370);
 assert.equal(resize.resizeKey(360,'ArrowRight',240,640,10,true),350);
 assert.equal(resize.resizeKey(360,'End',240,640),640);
 assert.equal(resize.resizeKey(360,'Home',240,640),240);
 assert.equal(resize.resizeKey(360,'Escape',240,640),null);
});
