export interface LearningCategory{id:string;name:string;description:string|null;icon:string|null;color:string|null;sortOrder:number}
export interface LearningSeries{id:string;categoryId:string|null;title:string;description:string|null;coverPath:string|null;sourceType:string|null;sourceUrl:string|null;sourcePlaylistId:string|null;author:string|null;totalItems:number;completedItems:number;totalDurationMs:number;favorite:boolean;sortOrder:number;createdAt:number;updatedAt:number}
export interface PlaybackProgress{itemId:string;positionMs:number;durationMs:number;completed:boolean;playbackRate:number;lastPlayedAt:number|null}
export interface LearningItem{id:string;seriesId:string;title:string;filePath:string|null;sourceUrl:string|null;sourceVideoId:string|null;sourcePlaylistIndex:number|null;durationMs:number|null;sortOrder:number;thumbnailPath:string|null;downloadTaskId:string|null;available:boolean;downloadState:string;progress:PlaybackProgress}
export interface VideoNote{id:string;itemId:string;timeMs:number;content:string;createdAt:number;updatedAt:number}
export interface LearningSettings{autoplayNext:boolean;completionThreshold:number;defaultPlaybackRate:number;restorePosition:boolean;autoSubtitles:boolean}
export interface SeriesDetail{series:LearningSeries;items:LearningItem[];page:number;total:number}
export interface ContinueLearningItem{series:LearningSeries;item:LearningItem}
export interface LibraryHome{categories:LearningCategory[];series:LearningSeries[];total:number;page:number;continueLearning:ContinueLearningItem[];settings:LearningSettings}
export interface CategoryInput{id?:string;name:string;description?:string|null;icon?:string|null;color?:string|null}
export interface SeriesInput{id?:string;categoryId:string|null;title:string;description?:string|null;coverPath?:string|null;sourceUrl?:string|null;favorite:boolean}
export interface ProgressInput{itemId:string;positionMs:number;durationMs:number;playbackRate:number;ended:boolean;sequence:number}
export interface ImportResult{seriesId:string;added:number;duplicates:number;errors:{path:string;error:string}[]}
export interface LocalScan{paths:string[];suggestedTitle:string;duplicates:number;existingSeriesId:string|null}

export interface PlayerSubtitle{label:string;path:string}
export interface PlayerMedia{item:LearningItem;series:LearningSeries;settings:LearningSettings;previous:string|null;next:string|null;path:string;subtitles:PlayerSubtitle[];subtitleWarning:string|null}

export interface NotePage{notes:VideoNote[];total:number;page:number}

import type {MediaPreview,DownloadRequest,DownloadTask} from '../types';
export interface LibraryChoice{enabled:boolean;title:string;seriesId:string|null;categoryId:string|null;keepOrder:boolean}
export interface PlaylistImport{preview:MediaPreview;seriesId:string|null;categoryId:string|null;title:string;keepOrder:boolean;template:DownloadRequest}
export interface LearningEnqueueResult{tasks:DownloadTask[];seriesIds:string[];added:number;duplicates:number}
