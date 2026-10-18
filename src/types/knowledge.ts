export interface KnowledgeSettings{vaultPath:string|null;vaultId:string|null;rootFolder:string;attachmentsFolder:string;obsidian?:ObsidianBinding|null}
export type ObsidianKind='vaultId'|'vaultName'|'absolutePath';
export interface ObsidianBinding{yingliuVaultId:string;kind:ObsidianKind;obsidianVaultIdentifier:string|null;registrationConfirmed:boolean;confirmedByUser:boolean}
export type ObsidianConnectionState='unconfigured'|'protocolUnavailable'|'registrationPending'|'verificationPending'|'userConfirmed'|'launchFailed'|'staleVault';
export interface ObsidianOpenResult{vaultId:string;path:string;status:'launchAccepted'|'launchFailed';message:string}
export interface ObsidianConnection{vaultId:string;vaultPath:string;binding:ObsidianBinding|null;state:ObsidianConnectionState;protocolAvailable:boolean;lastRequest:ObsidianOpenResult|null}
export interface KnowledgeView<T>{vaultId:string;data:T}
export interface KnowledgeDocument{path:string;text:string;hash:string;mtime:number}
export interface TreeEntry{path:string;name:string;directory:boolean}
export interface TreePage{items:TreeEntry[];total:number;page:number;skipped:number}
export interface KnowledgeWrite{vaultId:string;outcome:{status:'saved'|'conflict';document:KnowledgeDocument};indexWarning:string|null}
export interface KnowledgeMutation{vaultId:string;path:string;indexWarning:string|null}
export interface NoteCandidate{path:string;title:string}
export interface BacklinkPage{items:NoteCandidate[];total:number;page:number}
export interface SearchHit{kind:'note'|'transcript';path:string|null;title:string;excerpt:string;itemId:string|null;analysisId:string|null;startMs:number|null;endMs:number|null}
export interface SearchPage{items:SearchHit[];total:number;page:number}
export interface IndexStats{indexed:number;skipped:number;complete:boolean;warnings:string[]}
export interface KnowledgeAsset{mime:string;base64:string}
export type LeaveGuard=(leave:()=>void)=>void;

export interface KnowledgeReference{vaultId:string;path:string}
export interface KnowledgeOpenRequest extends KnowledgeReference{version:number}
