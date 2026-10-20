import type {PlaybackProgress} from '../../types/learning';
import type {PlayerAspect} from './playerInteraction';
export type NativeAction =
 | {type:'pause';paused:boolean} | {type:'seek';position:number} | {type:'rate';rate:number}
 | {type:'volume';volume:number} | {type:'mute';muted:boolean} | {type:'subtitle';id:number|null}
 | {type:'shortcuts';bindings:{key:string;id:string}[]}
 | {type:'loadSubtitle';path:string} | {type:'fullscreen';enabled:boolean}
 | {type:'audio';id:number|null} | {type:'frameStep';direction:'next'|'previous'} | {type:'aspect';ratio:PlayerAspect};
export interface NativeSnapshot {
 session:string;itemId:string;ready:boolean;position:number;duration:number;rate:number;
 paused:boolean;volume:number;muted:boolean;ended:boolean;
 audioTracks?:{id:number;label:string}[];audioId?:number|null;subtitles:{id:number;label:string}[];subtitleId:number|null;error:string|null;
}
// HTMLVideoElement already satisfies this shape; native playback supplies the same controls.
export interface PlaybackHandle {
 currentTime:number;duration:number;playbackRate:number;volume:number;muted:boolean;paused:boolean;
 play():Promise<void>;pause():void;
 persist?(ended?:boolean):Promise<PlaybackProgress>;
 control?(action:NativeAction):Promise<NativeSnapshot>;
}
