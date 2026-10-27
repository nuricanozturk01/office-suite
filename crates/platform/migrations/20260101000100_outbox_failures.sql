-- Rollback plan: DROP TABLE platform.outbox_failures (parked events stay marked as published).
-- Failure bookkeeping of the outbox relay; a row with `parked_at` is a dead letter that needs a human.
CREATE TABLE platform.outbox_failures (
    outbox     text        NOT NULL,
    event_id   uuid        NOT NULL,
    consumer   text        NOT NULL,
    attempts   integer     NOT NULL,
    last_error text        NOT NULL,
    parked_at  timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (outbox, event_id)
);
