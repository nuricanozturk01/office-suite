use async_trait::async_trait;
use documents_contracts::DocumentId;
use sqlx::{PgConnection, PgPool};

use crate::{
    application::{Loaded, StoreError, UpdateStore},
    domain::SyncState,
};

/// Advisory-lock namespace for compaction (first key of the two-key form).
const COMPACT_LOCK_NAMESPACE: i32 = 0x636F;

/// Postgres-backed update log (schema `collaboration`).
#[derive(Debug)]
pub(crate) struct PgUpdateStore {
    pool: PgPool,
}

impl PgUpdateStore {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn storage(err: &sqlx::Error) -> StoreError {
    tracing::error!(error = %err, "collaboration storage failure");
    StoreError
}

#[async_trait]
impl UpdateStore for PgUpdateStore {
    async fn load(&self, document: DocumentId) -> Result<Loaded, StoreError> {
        // One snapshot of the database: a compaction between the two reads must not hide rows.
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        repeatable_read(&mut tx).await?;
        load_from(&mut tx, document).await
    }

    async fn snapshot_after(
        &self,
        document: DocumentId,
        after: i64,
    ) -> Result<Option<(i64, Vec<u8>)>, StoreError> {
        let row = sqlx::query!(
            "SELECT state, upto_seq FROM collaboration.document_snapshots
             WHERE document_id = $1 AND upto_seq > $2",
            document.as_uuid(),
            after
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(row.map(|r| (r.upto_seq, r.state)))
    }

    async fn append(&self, document: DocumentId, payload: &[u8]) -> Result<i64, StoreError> {
        sqlx::query_scalar!(
            "INSERT INTO collaboration.document_updates (document_id, payload, created_at)
             VALUES ($1, $2, now()) RETURNING seq",
            document.as_uuid(),
            payload
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| storage(&e))
    }

    async fn updates_after(
        &self,
        document: DocumentId,
        after: i64,
    ) -> Result<Vec<(i64, Vec<u8>)>, StoreError> {
        let rows = sqlx::query!(
            "SELECT seq, payload FROM collaboration.document_updates
             WHERE document_id = $1 AND seq > $2 ORDER BY seq",
            document.as_uuid(),
            after
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(rows.into_iter().map(|r| (r.seq, r.payload)).collect())
    }

    async fn compact(&self, document: DocumentId) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        // Repeatable read: the delete below only sees rows the fold below has also seen, so a
        // row that commits late (its seq was assigned earlier) can never be deleted unfolded.
        repeatable_read(&mut tx).await?;
        let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1, hashtext($2))")
            .bind(COMPACT_LOCK_NAMESPACE)
            .bind(document.to_string())
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| storage(&e))?;
        if !locked {
            return Ok(());
        }
        let loaded = load_from(&mut tx, document).await?;
        let Some(&(upto_seq, _)) = loaded.updates.last() else {
            return Ok(());
        };
        // Folding is CPU work: keep it off the async runtime (CLAUDE.md §5.1).
        let (snapshot, updates) = (
            loaded.snapshot,
            loaded
                .updates
                .into_iter()
                .map(|(_, p)| p)
                .collect::<Vec<_>>(),
        );
        let state = tokio::task::spawn_blocking(move || fold(snapshot.as_deref(), &updates))
            .await
            .map_err(|_| StoreError)??;
        sqlx::query!(
            "INSERT INTO collaboration.document_snapshots (document_id, state, upto_seq, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (document_id)
             DO UPDATE SET state = EXCLUDED.state, upto_seq = EXCLUDED.upto_seq, updated_at = now()
             WHERE collaboration.document_snapshots.upto_seq < EXCLUDED.upto_seq",
            document.as_uuid(),
            state,
            upto_seq
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        sqlx::query!(
            "DELETE FROM collaboration.document_updates WHERE document_id = $1 AND seq <= $2",
            document.as_uuid(),
            upto_seq
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        tx.commit().await.map_err(|e| storage(&e))
    }
}

/// Folds a snapshot and updates into one state update.
fn fold(snapshot: Option<&[u8]>, updates: &[Vec<u8>]) -> Result<Vec<u8>, StoreError> {
    let sync = SyncState::new();
    if let Some(snapshot) = snapshot {
        sync.apply_update(snapshot).map_err(|_| StoreError)?;
    }
    for payload in updates {
        sync.apply_update(payload).map_err(|_| StoreError)?;
    }
    Ok(sync.encode_state())
}

/// Must be the first statement of the transaction.
async fn repeatable_read(tx: &mut PgConnection) -> Result<(), StoreError> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(tx)
        .await
        .map_err(|e| storage(&e))?;
    Ok(())
}

/// Snapshot plus every later update, read on the given connection.
async fn load_from(conn: &mut PgConnection, document: DocumentId) -> Result<Loaded, StoreError> {
    let snapshot = sqlx::query!(
        "SELECT state, upto_seq FROM collaboration.document_snapshots WHERE document_id = $1",
        document.as_uuid()
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| storage(&e))?;
    let after = snapshot.as_ref().map_or(0, |s| s.upto_seq);
    let updates = sqlx::query!(
        "SELECT seq, payload FROM collaboration.document_updates
         WHERE document_id = $1 AND seq > $2 ORDER BY seq",
        document.as_uuid(),
        after
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| storage(&e))?;
    Ok(Loaded {
        snapshot_seq: after,
        snapshot: snapshot.map(|s| s.state),
        updates: updates.into_iter().map(|r| (r.seq, r.payload)).collect(),
    })
}
