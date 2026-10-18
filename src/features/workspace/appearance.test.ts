import test from 'node:test';
import assert from 'node:assert/strict';
import {appearanceTokens} from './appearance.ts';
import {defaultWorkspace} from './defaults.ts';
test('appearance follows system preferences and retains readable text for extreme accents',()=>{
 const a=defaultWorkspace().appearance;a.theme='system';a.motion='system';a.fontSize=20;a.sidebarWidth=320;
 assert.equal(appearanceTokens(a,false,true).theme,'light');assert.equal(appearanceTokens(a,true,false).motion,'full');
 assert.equal(appearanceTokens(a,false,true).vars['--workspace-font-scale'],String(20/14));
 a.accent='#ffffff';const light=appearanceTokens(a,false,true);assert.equal(light.vars['--workspace-on-accent'],'#071119');assert.equal(light.vars['--workspace-accent-ink'],'#075985');
 a.accent='#000000';assert.equal(appearanceTokens(a,true,true).vars['--workspace-on-accent'],'#ffffff');assert.equal(appearanceTokens(a,true,true).vars['--workspace-accent-ink'],'#7dd3fc');
 a.theme='dark';a.motion='full';assert.equal(appearanceTokens(a,false,true).theme,'dark');assert.equal(appearanceTokens(a,false,true).motion,'full');
});
