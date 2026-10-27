-- Rollback plan: DROP TABLE storage.outbox (nothing references it; undelivered events are lost).
-- Integration events of the storage module (`PackageSaved`), written with the file in one transaction.
CREATE TABLE storage.outbox (
    id           uuid PRIMARY KEY,
    event_type   text        NOT NULL,
    payload      jsonb       NOT NULL,
    created_at   timestamptz NOT NULL,
    published_at timestamptz
);
CREATE INDEX outbox_unpublished_idx ON storage.outbox (created_at) WHERE published_at IS NULL;
