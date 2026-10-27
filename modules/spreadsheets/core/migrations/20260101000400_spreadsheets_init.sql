-- Rollback plan: DROP SCHEMA spreadsheets CASCADE (no other module references it).
CREATE TABLE spreadsheets.spreadsheets (
    id         uuid PRIMARY KEY,
    owner_id   uuid        NOT NULL,
    title      text        NOT NULL,
    -- Sparse workbook (sheets, cells, column widths). Validated in the domain before write.
    content    jsonb       NOT NULL,
    -- Optimistic lock for content writes. Renames do not bump it.
    version    bigint      NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    deleted_at timestamptz
);
CREATE INDEX spreadsheets_owner_idx ON spreadsheets.spreadsheets (owner_id, id DESC) WHERE deleted_at IS NULL;

CREATE TABLE spreadsheets.outbox (
    id           uuid PRIMARY KEY,
    event_type   text        NOT NULL,
    payload      jsonb       NOT NULL,
    created_at   timestamptz NOT NULL,
    published_at timestamptz
);
CREATE INDEX outbox_unpublished_idx ON spreadsheets.outbox (created_at) WHERE published_at IS NULL;
