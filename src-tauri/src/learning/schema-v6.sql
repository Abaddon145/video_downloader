CREATE TABLE video_analyses(
 id TEXT PRIMARY KEY,item_id TEXT REFERENCES series_items(id) ON DELETE SET NULL,
 source_path TEXT NOT NULL,source_fingerprint TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('queued','extracting_audio','transcribing','extracting_keyframes','analyzing_chunks','finalizing','writing_note','completed','failed','cancelled','interrupted')),
 analysis_mode TEXT NOT NULL CHECK(analysis_mode IN ('quick','standard','deep')),
 model TEXT NOT NULL,prompt_version TEXT NOT NULL,language TEXT,duration_ms INTEGER CHECK(duration_ms>=0),
 progress REAL NOT NULL DEFAULT 0 CHECK(progress>=0 AND progress<=1),stage TEXT,
 summary_json TEXT,markdown_path TEXT,markdown_vault_id TEXT,error TEXT,
 transcript_done INTEGER NOT NULL DEFAULT 0,keyframes_done INTEGER NOT NULL DEFAULT 0,
 created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,completed_at INTEGER);
CREATE INDEX idx_analyses_item ON video_analyses(item_id);
CREATE INDEX idx_analyses_cache ON video_analyses(source_fingerprint,prompt_version,model,analysis_mode);
CREATE INDEX idx_analyses_status ON video_analyses(status,created_at);
CREATE TABLE transcript_segments(
 id TEXT PRIMARY KEY,analysis_id TEXT NOT NULL REFERENCES video_analyses(id) ON DELETE CASCADE,
 segment_index INTEGER NOT NULL CHECK(segment_index>=0),start_ms INTEGER NOT NULL CHECK(start_ms>=0),
 end_ms INTEGER NOT NULL CHECK(end_ms>start_ms),text TEXT NOT NULL,
 UNIQUE(analysis_id,segment_index));
CREATE INDEX idx_transcript_time ON transcript_segments(analysis_id,start_ms);
CREATE TABLE analysis_chunks(
 id TEXT PRIMARY KEY,analysis_id TEXT NOT NULL REFERENCES video_analyses(id) ON DELETE CASCADE,
 chunk_index INTEGER NOT NULL CHECK(chunk_index>=0),start_ms INTEGER NOT NULL CHECK(start_ms>=0),
 end_ms INTEGER NOT NULL CHECK(end_ms>start_ms),result_json TEXT,
 status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed')),
 UNIQUE(analysis_id,chunk_index));
CREATE TABLE analysis_keyframes(
 id TEXT PRIMARY KEY,analysis_id TEXT NOT NULL REFERENCES video_analyses(id) ON DELETE CASCADE,
 time_ms INTEGER NOT NULL CHECK(time_ms>=0),path TEXT NOT NULL,sha256 TEXT NOT NULL,scene_score REAL);
CREATE INDEX idx_keyframes_time ON analysis_keyframes(analysis_id,time_ms);
CREATE TABLE ai_usage(
 id TEXT PRIMARY KEY,analysis_id TEXT REFERENCES video_analyses(id) ON DELETE SET NULL,
 operation TEXT NOT NULL,model TEXT NOT NULL,input_tokens INTEGER CHECK(input_tokens>=0),
 output_tokens INTEGER CHECK(output_tokens>=0),cached_tokens INTEGER CHECK(cached_tokens>=0),
 usage_json TEXT,created_at INTEGER NOT NULL);
CREATE INDEX idx_usage_time ON ai_usage(created_at);
CREATE TABLE knowledge_index(
 path TEXT PRIMARY KEY,title TEXT NOT NULL,content_hash TEXT NOT NULL,mtime INTEGER NOT NULL,
 content TEXT NOT NULL,analysis_id TEXT REFERENCES video_analyses(id) ON DELETE SET NULL,
 item_id TEXT REFERENCES series_items(id) ON DELETE SET NULL,indexed_at INTEGER NOT NULL);
CREATE TABLE knowledge_links(
 source_path TEXT NOT NULL REFERENCES knowledge_index(path) ON DELETE CASCADE ON UPDATE CASCADE,
 target TEXT NOT NULL,PRIMARY KEY(source_path,target));
CREATE INDEX idx_knowledge_target ON knowledge_links(target);
CREATE TABLE knowledge_tags(
 path TEXT NOT NULL REFERENCES knowledge_index(path) ON DELETE CASCADE ON UPDATE CASCADE,
 tag TEXT NOT NULL,PRIMARY KEY(path,tag));
CREATE INDEX idx_knowledge_tag ON knowledge_tags(tag);
UPDATE schema_version SET version=6;
