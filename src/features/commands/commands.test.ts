import test from 'node:test';
import assert from 'node:assert/strict';
import {CommandRegistry, searchCommands} from './CommandRegistry.ts';
import {normalizeShortcut, shortcutIssue, importShortcuts, shouldHandleShortcut, eventShortcut, ShortcutManager} from './ShortcutManager.ts';

const command=(id:string,extra:Record<string,unknown>={})=>({id,title:id,category:'导航',execute:()=>{},...extra});
test('registration cleanup never removes a newer owner and restores previous owner',()=>{
 const registry=new CommandRegistry();const first=registry.register([command('note.new')]);const second=registry.register([command('note.new',{title:'当前笔记'})]);
 first();assert.equal(registry.getSnapshot().commands[0].title,'当前笔记');second();assert.equal(registry.getSnapshot().commands.length,0);
 const old=registry.register([command('note.new')]);const current=registry.register([command('note.new',{title:'新'})]);current();assert.equal(registry.getSnapshot().commands[0].title,'note.new');old();
});
test('run checks availability, deduplicates async work and records only successful ids/count/time',async()=>{
 const saved:string[]=[];const registry=new CommandRegistry({getItem:()=>null,setItem:(_k,v)=>saved.push(v)});let calls=0;let finish!:()=>void;
 registry.register([command('note.new',{execute:()=>{calls++;return new Promise<void>(resolve=>finish=resolve);}}),command('note.disabled',{enabled:false,availabilityReason:'请先打开笔记'}),command('note.failed',{execute:()=>{throw new Error('保存失败');}})]);
 assert.equal((await registry.run('note.disabled')).ok,false);assert.equal(calls,0);
 const pending=registry.run('note.new');assert.equal((await registry.run('note.new')).ok,false);assert.equal(calls,1);finish();assert.equal((await pending).ok,true);
 assert.equal((await registry.run('note.failed')).ok,false);assert.match(registry.getSnapshot().error,/保存失败/);
 assert.deepEqual(Object.keys(JSON.parse(saved.at(-1)!)[0]).sort(),['count','id','time']);assert.equal(registry.getSnapshot().recent.length,1);
});
test('late command results cannot update the error for a newly registered target',async()=>{
 const registry=new CommandRegistry();let reject!:(error:Error)=>void;
 const clean=registry.register([command('note.save',{execute:()=>new Promise<void>((_ok,no)=>reject=no)})]);const pending=registry.run('note.save');clean();registry.register([command('note.save')]);reject(new Error('旧笔记失败'));await pending;assert.equal(registry.getSnapshot().error,'');
});
test('search ranks Chinese/English keywords, fuzzy input and recent usage',()=>{
 const commands=[command('nav.download'),command('note.new',{title:'新建笔记',keywords:['note','create']})];
 assert.equal(searchCommands(commands,'笔记')[0].id,'note.new');assert.equal(searchCommands(commands,'nte')[0].id,'note.new');assert.equal(searchCommands(commands,'note')[0].id,'note.new');
 assert.equal(searchCommands(commands,'',[{id:'note.new',time:10,count:2}])[0].id,'note.new');assert.equal(searchCommands(commands,'不存在').length,0);
});
test('shortcut normalization and conflict checks include reserved and overlapping scopes',()=>{
 assert.equal(normalizeShortcut('shift + ctrl + k'),'Ctrl+Shift+K');assert.equal(normalizeShortcut('Space'),'Space');assert.equal(normalizeShortcut('Ctrl+Ctrl+K'),null);
 assert.match(shortcutIssue('Ctrl+Z','knowledge',[],'note.undo')!,/编辑器/);assert.match(shortcutIssue('Alt+F4','global',[],'app.close')!,/系统/);
 assert.equal(shortcutIssue('J','player',[],'player.back'),null);assert.match(shortcutIssue('J','global',[],'app.back')!,/修饰/);
 assert.match(shortcutIssue('Ctrl+K','player',[{id:'app.palette',shortcut:'Ctrl+K',scope:'global'}],'player.back')!,/冲突/);
 assert.equal(shortcutIssue('J','knowledge',[{id:'player.back',shortcut:'J',scope:'player'}],'note.back'),null);
});
test('import validates size, exact schema, known command ids and reserved shortcuts',()=>{
 const commands=[command('note.new',{shortcut:'Ctrl+Shift+N'})];assert.deepEqual({...importShortcuts('{"version":1,"bindings":{"note.new":"Ctrl+Alt+N"}}',commands)},{'note.new':'Ctrl+Alt+N'});
 for(const text of ['{"version":1,"apiKey":"secret","bindings":{}}','{"version":1,"bindings":{"unknown.id":"Ctrl+N"}}','{"version":1,"bindings":{"note.new":"Ctrl+Z"}}',' '.repeat(131073)])assert.throws(()=>importShortcuts(text,commands));
});
test('shortcut dispatch respects composition, repeat, editable focus, overlays and scope',()=>{
 const c=command('player.pause',{shortcut:'Space',scope:'player'});
 const base={key:' ',ctrlKey:false,altKey:false,shiftKey:false,metaKey:false,isComposing:false,repeat:false,defaultPrevented:false};assert.equal(eventShortcut(base),'Space');
 assert.equal(shouldHandleShortcut(base,c,{scope:'player',editable:false,overlay:false}),true);
 for(const field of ['isComposing','repeat','defaultPrevented'])assert.equal(shouldHandleShortcut({...base,[field]:true},c,{scope:'player',editable:false,overlay:false}),false);
 assert.equal(shouldHandleShortcut(base,c,{scope:'player',editable:true,overlay:false}),false);assert.equal(shouldHandleShortcut(base,c,{scope:'global',editable:false,overlay:false}),false);assert.equal(shouldHandleShortcut(base,c,{scope:'player',editable:false,overlay:true}),false);
});
test('shortcut manager attaches only one listener and cleanup is idempotent',()=>{
 const registry=new CommandRegistry();let adds=0,removes=0;const target={addEventListener(){adds++;},removeEventListener(){removes++;}};const manager=new ShortcutManager(registry);
 const off=manager.attach(target as any);const offAgain=manager.attach(target as any);assert.equal(adds,1);offAgain();off();assert.equal(removes,1);
});

test('category words are searchable and recent records stay capped at thirty',async()=>{
 const registry=new CommandRegistry();registry.register(Array.from({length:35},(_,index)=>command(`note.${index}`,{category:'知识库'})));
 assert.equal(searchCommands(registry.getSnapshot().commands,'知识库').length,35);
 for(let index=0;index<35;index++)await registry.run(`note.${index}`);
 assert.equal(registry.getSnapshot().recent.length,30);assert.equal(registry.getSnapshot().recent[0].id,'note.34');
});
test('configuration changes are atomic when storage fails and editor save is allowed explicitly',()=>{
 const registry=new CommandRegistry();registry.register([command('note.save',{shortcut:'Ctrl+S',scope:'knowledge',allowInEditable:true})]);
 const manager=new ShortcutManager(registry,{getItem:()=>null,setItem:()=>{throw new Error('磁盘不可写');}});
 assert.throws(()=>manager.setBinding('note.save','Ctrl+Alt+S'));assert.equal(manager.binding(registry.getSnapshot().commands[0]),'Ctrl+S');
 assert.equal(shouldHandleShortcut({key:'s',ctrlKey:true,altKey:false,shiftKey:false,metaKey:false},registry.getSnapshot().commands[0],{scope:'knowledge',editable:true,overlay:false}),true);
});

test('late registration conflict suppresses dispatch and remount removes every listener',()=>{
 const registry=new CommandRegistry();let calls=0;registry.register([command('note.save',{shortcut:'Ctrl+S',scope:'knowledge',execute:()=>{calls++;}})]);
 const manager=new ShortcutManager(registry);manager.setBinding('note.save','Ctrl+Alt+S');
 const remove=registry.register([command('app.other',{shortcut:'Ctrl+Alt+S'})]);assert.match(manager.issue('note.save','Ctrl+Alt+S')!,/冲突/);
 let handler:(event:any)=>void=()=>{};let adds=0,removes=0;const target={addEventListener(_type:string,next:any){handler=next;adds++;},removeEventListener(){removes++;}};
 const off=manager.attach(target as any);handler({key:'s',ctrlKey:true,altKey:true,metaKey:false,shiftKey:false,target:{closest:()=>null},preventDefault(){throw new Error('conflict was consumed');}});assert.equal(calls,0);off();
 remove();assert.equal(manager.issue('note.save','Ctrl+Alt+S'),null);const off2=manager.attach(target as any);off2();off2();assert.equal(adds,2);assert.equal(removes,2);
});
test('shifted digit keyboard events match custom numeric shortcuts',()=>{
 assert.equal(eventShortcut({key:'!',code:'Digit1',ctrlKey:true,shiftKey:true,altKey:false,metaKey:false}), 'Ctrl+Shift+1');
});
test('opening the command UI never buries recent business commands',async()=>{
 const registry=new CommandRegistry();registry.register([{id:'navigate.history',title:'历史',category:'导航',execute:()=>{}},{id:'app.palette',title:'面板',category:'全局',execute:()=>{}}]);
 await registry.run('navigate.history');await registry.run('app.palette');
 assert.equal(registry.getSnapshot().recent[0].id,'navigate.history');
});
test('Space on a focused action button is left to the button',()=>{
 assert.equal(shouldHandleShortcut({key:' ',ctrlKey:false,shiftKey:false,altKey:false,metaKey:false},{id:'player.toggle',title:'播放',category:'播放器',shortcut:'Space',scope:'player',execute:()=>{}},{scope:'player',editable:false,overlay:false,interactive:true}),false);
});
