-- Rollback plan: DROP SCHEMA documents CASCADE (no other module references it).
CREATE TABLE documents.documents (
    id         uuid PRIMARY KEY,
    owner_id   uuid        NOT NULL,
    title      text        NOT NULL,
    version    bigint      NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    deleted_at timestamptz
);
-- Serves the owner listing (keyset pagination on id, UUID v7 is time-ordered).
CREATE INDEX documents_owner_idx ON documents.documents (owner_id, id DESC) WHERE deleted_at IS NULL;

CREATE TABLE documents.outbox (
    id           uuid PRIMARY KEY,
    event_type   text        NOT NULL,
    payload      jsonb       NOT NULL,
    created_at   timestamptz NOT NULL,
    published_at timestamptz
);
CREATE INDEX outbox_unpublished_idx ON documents.outbox (created_at) WHERE published_at IS NULL;
