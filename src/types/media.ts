export type MediaOperation='remux'|'transcode'|'compress'|'trim'|'extractAudio'|'screenshot';
export type OutputFormat='mp4'|'mkv'|'mov'|'webm'|'mp3'|'m4a'|'aac'|'wav'|'flac'|'opus'|'png'|'jpg'|'webp';
export type MediaStatus='queued'|'probing'|'processing'|'interrupted'|'completed'|'failed'|'cancelled';
export interface VideoStreamInfo {index:number;codec:string;profile:string|null;width:number|null;height:number|null;fps:number|null;pixelFormat:string|null;bitDepth:number|null;bitrate:number|null;attachedPicture:boolean;metadata:Record<string,string>}
export interface AudioStreamInfo {index:number;codec:string;profile:string|null;sampleRate:number|null;channels:number|null;channelLayout:string|null;bitrate:number|null;metadata:Record<string,string>}
export interface SubtitleStreamInfo {index:number;codec:string;language:string|null;metadata:Record<string,string>}
export interface MediaInfo {fileName:string;path:string;size:number|null;duration:number|null;container:string;bitrate:number|null;videos:VideoStreamInfo[];audios:AudioStreamInfo[];subtitles:SubtitleStreamInfo[];otherStreams:number;metadata:Record<string,string>}
export interface MediaRequest {inputPath:string;outputDir:string;operation:MediaOperation;outputFormat:OutputFormat;videoCodec:'h264'|'hevc'|'copy';audioCodec:'aac'|'mp3'|'opus'|'copy';quality:'high'|'balanced'|'small';crf:number|null;height:number|null;fps:number|null;audioBitrate:number;start:number;end:number|null;trimMode:'fast'|'accurate';copyAudio:boolean;hardwareAcceleration?:HardwareAcceleration;generateThumbnails?:boolean;sourceDownloadId?:string|null}
export interface MediaTask {id:string;type:MediaOperation;inputPath:string;outputPath:string|null;request:MediaRequest;status:MediaStatus;phase:string;progress:number|null;speed:number|null;processedTime:number|null;totalDuration:number|null;eta:number|null;error:string|null;logs:string[];createdAt:number;finishedAt:number|null}
export interface MediaSnapshot {tasks:MediaTask[];ready:boolean;error:string|null}
export interface MediaForm {operation:MediaOperation;outputFormat:OutputFormat;videoCodec?:MediaRequest['videoCodec'];audioCodec?:MediaRequest['audioCodec'];quality?:MediaRequest['quality'];crf?:number;height?:number;fps?:number;audioBitrate?:number;start?:string;end?:string;trimMode?:MediaRequest['trimMode'];copyAudio?:boolean;advanced?:boolean;hardwareAcceleration?:HardwareAcceleration;generateThumbnails?:boolean}

export type HardwareAcceleration='auto'|'cpu'|'nvidia'|'intel'|'amd';
export interface MediaPreset {id:string;name:string;operation:MediaOperation;outputFormat:OutputFormat;videoCodec:MediaRequest['videoCodec'];audioCodec:MediaRequest['audioCodec'];resolution:number|null;quality:MediaRequest['quality'];hardwareAcceleration:HardwareAcceleration;generateThumbnails:boolean}
export interface MediaProSettings {hardwareAcceleration:HardwareAcceleration;presets:MediaPreset[];afterDownload:'none'|'mp4'|'compress'|'subtitles'|'cover';enabledAt:number}
