-- Rollback plan: DROP TABLE storage.package_history (history is derived; packages stay intact).
-- Every saved version of a package, newest 50 per resource kept by the application.
CREATE TABLE storage.package_history (
    resource_id uuid        NOT NULL,
    version     bigint      NOT NULL,
    name        text        NOT NULL,
    bytes       bytea       NOT NULL,
    saved_by    uuid        NOT NULL,
    saved_at    timestamptz NOT NULL,
    PRIMARY KEY (resource_id, version)
);

-- Existing packages start their history at the version they have now.
INSERT INTO storage.package_history (resource_id, version, name, bytes, saved_by, saved_at)
SELECT resource_id, version, name, bytes, updated_by, updated_at FROM storage.packages;
