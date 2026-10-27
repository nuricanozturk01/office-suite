-- Rollback: ALTER TABLE identity.users DROP COLUMN avatar_url, DROP COLUMN bio, DROP COLUMN deletion_scheduled_at.
ALTER TABLE identity.users ADD COLUMN avatar_url            text;
ALTER TABLE identity.users ADD COLUMN bio                   text;
ALTER TABLE identity.users ADD COLUMN deletion_scheduled_at timestamptz;
