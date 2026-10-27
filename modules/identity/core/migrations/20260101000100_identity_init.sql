-- Rollback plan: DROP SCHEMA identity CASCADE (no other module references it).
CREATE TABLE identity.users (
    id            uuid PRIMARY KEY,
    email         text        NOT NULL,
    display_name  text        NOT NULL,
    password_hash text        NOT NULL,
    created_at    timestamptz NOT NULL,
    updated_at    timestamptz NOT NULL
);
CREATE UNIQUE INDEX users_email_key ON identity.users (email);

CREATE TABLE identity.outbox (
    id           uuid PRIMARY KEY,
    event_type   text        NOT NULL,
    payload      jsonb       NOT NULL,
    created_at   timestamptz NOT NULL,
    published_at timestamptz
);
CREATE INDEX outbox_unpublished_idx ON identity.outbox (created_at) WHERE published_at IS NULL;
