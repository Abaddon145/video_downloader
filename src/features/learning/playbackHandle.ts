import type {PlaybackProgress} from '../../types/learning';
export type NativeAction =
 | {type:'pause';paused:boolean} | {type:'seek';position:number} | {type:'rate';rate:number}
 | {type:'volume';volume:number} | {type:'mute';muted:boolean} | {type:'subtitle';id:number|null}
 | {type:'loadSubtitle';path:string} | {type:'fullscreen';enabled:boolean};
export interface NativeSnapshot {
 session:string;itemId:string;ready:boolean;position:number;duration:number;rate:number;
 paused:boolean;volume:number;muted:boolean;ended:boolean;
 subtitles:{id:number;label:string}[];subtitleId:number|null;error:string|null;
}
// HTMLVideoElement already satisfies this shape; native playback supplies the same controls.
export interface PlaybackHandle {
 currentTime:number;duration:number;playbackRate:number;volume:number;muted:boolean;paused:boolean;
 play():Promise<void>;pause():void;
 persist?(ended?:boolean):Promise<PlaybackProgress>;
 control?(action:NativeAction):Promise<NativeSnapshot>;
}
