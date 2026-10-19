CREATE TABLE knowledge_note_metadata(
 vault_id TEXT NOT NULL,path TEXT NOT NULL,
 favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)),
 template INTEGER NOT NULL DEFAULT 0 CHECK(template IN (0,1)),
 PRIMARY KEY(vault_id,path));
CREATE TABLE knowledge_note_versions(
 id TEXT PRIMARY KEY,vault_id TEXT NOT NULL,path TEXT NOT NULL,
 content_hash TEXT NOT NULL,text TEXT NOT NULL,
 bytes INTEGER NOT NULL CHECK(bytes BETWEEN 0 AND 5242880),created_at INTEGER NOT NULL);
CREATE INDEX idx_note_versions ON knowledge_note_versions(vault_id,path,created_at,id);
CREATE TABLE knowledge_attachments(
 vault_id TEXT NOT NULL,path TEXT NOT NULL,name TEXT NOT NULL,
 content_hash TEXT NOT NULL,bytes INTEGER NOT NULL CHECK(bytes BETWEEN 0 AND 20971520),
 imported_at INTEGER NOT NULL,PRIMARY KEY(vault_id,path));
UPDATE schema_version SET version=8;
