ALTER TABLE series_items ADD COLUMN thumbnail_source TEXT CHECK(thumbnail_source IS NULL OR thumbnail_source IN ('manual','remote','generated','series','placeholder'));
ALTER TABLE series_items ADD COLUMN source_thumbnail_url TEXT;
ALTER TABLE series_items ADD COLUMN thumbnail_key TEXT;
UPDATE series_items SET source_thumbnail_url=thumbnail_path,thumbnail_path=NULL WHERE lower(thumbnail_path) LIKE 'http://%' OR lower(thumbnail_path) LIKE 'https://%';
UPDATE series_items SET thumbnail_source='manual' WHERE thumbnail_path IS NOT NULL;
UPDATE schema_version SET version=5;
