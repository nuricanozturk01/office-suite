-- Serves the owner listing ordered by last update (keyset on updated_at, id).
-- Rollback plan: DROP INDEX presentations.presentations_owner_updated_idx; the listing then sorts without an index.
CREATE INDEX presentations_owner_updated_idx ON presentations.presentations (owner_id, updated_at DESC, id DESC) WHERE deleted_at IS NULL;
