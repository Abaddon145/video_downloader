import type {TranscriptSegment,VideoAnalysisResult} from '../../types/ai.ts';
import {timeLabel} from './lib.ts';

const escapeMarkdown=(text:string)=>text.replace(/[\\`*_{}\[\]()#+.!|>~-]/g,'\\$&');
const sourceLink=(itemId:string,ms:number)=>`[${timeLabel(ms)}](yingliu://video/${encodeURIComponent(itemId)}?t=${ms})`;

export function subtitleMarkdown(itemId:string,segments:TranscriptSegment[]):string{
 return segments.map(segment=>{
  const quote=segment.text.trim().split(/\r?\n/).map(line=>`> ${escapeMarkdown(line)}`).join('\n');
  return `${sourceLink(itemId,segment.startMs)}–${timeLabel(segment.endMs)} · 字幕原文\n<!-- itemId:${encodeURIComponent(itemId)} segmentId:${encodeURIComponent(segment.id)} startMs:${segment.startMs} endMs:${segment.endMs} -->\n${quote}`;
 }).join('\n\n');
}

export function summaryMarkdown(itemId:string,title:string,summary:VideoAnalysisResult):string{
 const chapters=summary.chapters.map(chapter=>`- ${sourceLink(itemId,chapter.startMs)} ${escapeMarkdown(chapter.title)}：${escapeMarkdown(chapter.summary)}`).join('\n');
 return `## ${escapeMarkdown(title)} · AI 总结\n\n${escapeMarkdown(summary.oneSentenceSummary)}\n\n${escapeMarkdown(summary.overview)}${chapters?`\n\n### 章节\n${chapters}`:''}`;
}

export function appendKnowledge(original:string,addition:string,reviewedHash:string,currentHash:string):string|null{
 const newline=original.includes('\r\n')?'\r\n':'\n';
 return reviewedHash===currentHash?`${original}${newline}${newline}${addition.trim()}${newline}`:null;
}

export function safeNoteName(title:string):string{
 const name=title.replace(/[<>:"/\\|?*\[\]#\x00-\x1f]/g,' ').trim().replace(/[. ]+$/,'').slice(0,60).trim();
 return `${name||'课程摘录'}-${Date.now().toString(36)}.md`;
}
