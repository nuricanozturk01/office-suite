-- Rollback plan: DROP TABLE identity.refresh_tokens (every session then has to sign in again).
-- Rotating refresh tokens. Only the SHA-256 of a token is stored. Tokens of one login share a
-- family; presenting an already-used token revokes the whole family (theft detection).
CREATE TABLE identity.refresh_tokens (
    id         uuid PRIMARY KEY,
    family_id  uuid        NOT NULL,
    user_id    uuid        NOT NULL,
    token_hash bytea       NOT NULL,
    expires_at timestamptz NOT NULL,
    used_at    timestamptz,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL
);
-- Serves the lookup of a presented token.
CREATE UNIQUE INDEX refresh_tokens_hash_key ON identity.refresh_tokens (token_hash);
-- Serves family revocation.
CREATE INDEX refresh_tokens_family_idx ON identity.refresh_tokens (family_id);
