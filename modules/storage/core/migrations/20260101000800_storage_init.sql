-- Rollback plan: DROP SCHEMA storage CASCADE (no other module references it).
-- The file package behind a Docs, Sheets, Slides or PDF resource, stored as its native format.
CREATE TABLE storage.packages (
    resource_id uuid PRIMARY KEY,
    name        text        NOT NULL,
    bytes       bytea       NOT NULL,
    -- Optimistic lock: a save names the version it was based on.
    version     bigint      NOT NULL DEFAULT 1,
    updated_by  uuid        NOT NULL,
    created_at  timestamptz NOT NULL,
    updated_at  timestamptz NOT NULL
);
