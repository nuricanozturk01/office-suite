-- Serves the owner listing ordered by last update (keyset on updated_at, id).
-- Rollback plan: DROP INDEX pdfs.pdfs_owner_updated_idx; the listing then sorts without an index.
CREATE INDEX pdfs_owner_updated_idx ON pdfs.pdfs (owner_id, updated_at DESC, id DESC) WHERE deleted_at IS NULL;
