-- Rollback plan: DROP SCHEMA platform CASCADE (consumers would re-process events afterwards).
-- Idempotency ledger shared by all durable event consumers; keyed by consumer name.
CREATE TABLE platform.processed_events (
    consumer     text        NOT NULL,
    event_id     uuid        NOT NULL,
    processed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (consumer, event_id)
);
