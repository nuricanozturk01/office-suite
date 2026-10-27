use uuid::Uuid;

/// Source of fresh identifiers; injected so tests are deterministic.
pub trait IdGenerator: Send + Sync {
    /// Returns a new unique UUID.
    fn next(&self) -> Uuid;
}

/// UUID v7 (time-ordered) generator.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidV7Generator;

impl IdGenerator for UuidV7Generator {
    fn next(&self) -> Uuid {
        Uuid::now_v7()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v7_ids_are_unique_and_versioned() {
        let generator = UuidV7Generator;
        let (a, b) = (generator.next(), generator.next());
        assert_ne!(a, b);
        assert_eq!(a.get_version_num(), 7);
    }
}
