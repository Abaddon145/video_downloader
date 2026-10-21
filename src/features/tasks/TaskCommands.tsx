import {useTaskCenter} from './TaskCenter';
import {useRegisterCommands} from '../commands/CommandProvider';
export function TaskCommands(){
 const {open}=useTaskCenter();
 useRegisterCommands([{id:'tasks.open',title:'查看后台任务',category:'任务',keywords:['task','background','下载','AI','转换'],execute:()=>open()},{id:'tasks.failed',title:'查看失败任务',category:'任务',keywords:['error','failed','错误'],execute:()=>open('failed')}]);
 return null;
}
