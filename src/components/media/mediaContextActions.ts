import type {MediaTask} from '../../types/media';

type Task=Pick<MediaTask,'id'|'inputPath'|'outputPath'|'status'>;
const playable=/\.(mp4|m4v|mov|mkv|avi|wmv|asf|webm|flv|mpg|mpeg|ts|m2ts|mts|3gp|3g2|ogv|mp3|aac|flac|wav|m4a|opus|ogg|wma)$/i;

export const mediaContextIds=(clickedId:string,selected:Set<string>)=>selected.has(clickedId)?[...selected]:[clickedId];
export const canRemoveMediaRecords=(tasks:Task[])=>tasks.length>0&&tasks.every(task=>['completed','failed','cancelled','interrupted'].includes(task.status));
export const batchInputPaths=(tasks:Task[])=>[...new Set(tasks.map(task=>[task.outputPath,task.inputPath].find(path=>path&&playable.test(path))).filter((path):path is string=>!!path))];
