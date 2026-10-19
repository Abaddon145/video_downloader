ALTER TABLE video_analyses ADD COLUMN template_id TEXT NOT NULL DEFAULT 'general';
ALTER TABLE video_analyses ADD COLUMN template_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE video_analyses ADD COLUMN template_snapshot TEXT;
ALTER TABLE video_analyses ADD COLUMN template_warning TEXT;
ALTER TABLE video_analyses ADD COLUMN reuse_analysis_id TEXT;
CREATE TABLE ai_template_versions(
 id TEXT NOT NULL,version INTEGER NOT NULL CHECK(version>0),definition_json TEXT NOT NULL,
 created_at INTEGER NOT NULL,PRIMARY KEY(id,version));
CREATE TABLE ai_template_preferences(
 id TEXT PRIMARY KEY,favorite INTEGER NOT NULL DEFAULT 0,deleted INTEGER NOT NULL DEFAULT 0);
UPDATE schema_version SET version=7;
