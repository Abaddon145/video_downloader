export type LibraryView = 'cards' | 'list' | 'table';
export type PlayerLayout = 'standard' | 'stacked' | 'theater' | 'focus';
export type LibrarySort = 'manual' | 'title' | 'updated' | 'lastPlayed' | 'progress' | 'duration';
export interface WorkspaceLayout {
 library: {view:LibraryView;categoryWidth:number;categoryCollapsed:boolean;cardSize:number;cardsPerRow:number|null;coverRatio:'16:9'|'4:3'|'square';showProgress:boolean;showAiStatus:boolean;showContinue:boolean;sort:LibrarySort;columns:string[];columnWidths:Record<string,number>};
 player: {layout:PlayerLayout;panelSize:number;panelCollapsed:boolean};
 knowledge: {treeWidth:number;infoWidth:number;treeCollapsed:boolean;infoCollapsed:boolean};
}
export interface WorkspaceAppearance {theme:'dark'|'light'|'system';accent:string;fontSize:number;density:'comfortable'|'standard'|'compact';sidebarWidth:number;motion:'system'|'reduced'|'full'}
export interface WorkspacePreset {id:string;name:string;layout:WorkspaceLayout}
export interface WorkspaceSettings {version:1;appearance:WorkspaceAppearance;currentLayout:WorkspaceLayout;presets:WorkspacePreset[]}
export type LibraryLayout = WorkspaceLayout['library'];
export type PlayerLayoutSettings = WorkspaceLayout['player'];
