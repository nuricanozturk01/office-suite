-- Rollback plan: DROP SCHEMA workspace CASCADE (the resource read-model rebuilds from events;
-- shares would be lost, so take a backup first).

-- Read-model of shareable resources, fed by document/spreadsheet events. Rows may be stubs
-- (owner_id/kind NULL) when a rename or delete event arrives before the created event.
CREATE TABLE workspace.resources (
    id         uuid PRIMARY KEY,
    kind       text,
    owner_id   uuid,
    title      text        NOT NULL DEFAULT '',
    title_at   timestamptz,
    created_at timestamptz,
    deleted_at timestamptz
);

CREATE TABLE workspace.shares (
    resource_id uuid        NOT NULL,
    grantee_id  uuid        NOT NULL,
    role        text        NOT NULL CHECK (role IN ('viewer', 'commenter', 'editor')),
    shared_by   uuid        NOT NULL,
    created_at  timestamptz NOT NULL,
    updated_at  timestamptz NOT NULL,
    PRIMARY KEY (resource_id, grantee_id)
);
-- Serves "shared with me" (keyset pagination on resource id; UUID v7 is time-ordered).
CREATE INDEX shares_grantee_idx ON workspace.shares (grantee_id, resource_id DESC);

CREATE TABLE workspace.outbox (
    id           uuid PRIMARY KEY,
    event_type   text        NOT NULL,
    payload      jsonb       NOT NULL,
    created_at   timestamptz NOT NULL,
    published_at timestamptz
);
CREATE INDEX outbox_unpublished_idx ON workspace.outbox (created_at) WHERE published_at IS NULL;
