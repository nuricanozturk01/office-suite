-- Rollback plan: DROP SCHEMA collaboration CASCADE (content is lost; take a backup first).
-- Append-only Yjs updates; compaction folds them into the snapshot and deletes folded rows.
CREATE TABLE collaboration.document_updates (
    seq         bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    document_id uuid        NOT NULL,
    payload     bytea       NOT NULL,
    created_at  timestamptz NOT NULL
);
-- Serves loading a document's updates in order.
CREATE INDEX document_updates_doc_idx ON collaboration.document_updates (document_id, seq);

CREATE TABLE collaboration.document_snapshots (
    document_id uuid PRIMARY KEY,
    state       bytea       NOT NULL,
    upto_seq    bigint      NOT NULL,
    updated_at  timestamptz NOT NULL
);
