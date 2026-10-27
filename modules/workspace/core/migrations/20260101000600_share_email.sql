-- Rollback plan: ALTER TABLE workspace.shares DROP COLUMN grantee_email (display data only).
-- The email the owner typed when sharing (normalized), shown in the share dialog. The identity
-- module stays the source of truth for accounts; this is a denormalized label.
ALTER TABLE workspace.shares ADD COLUMN grantee_email text NOT NULL DEFAULT '';
