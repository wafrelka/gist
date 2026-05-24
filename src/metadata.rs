use chrono::{DateTime, Utc};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Metadata {
    pub fn from_created_at(created_at: DateTime<Utc>) -> Self {
        Self { created_at, archived_at: None }
    }

    pub fn archived(&self, archived_at: DateTime<Utc>) -> Self {
        Self { archived_at: Some(archived_at), ..*self }
    }

    pub fn unarchived(&self) -> Self {
        Self { archived_at: None, ..*self }
    }

    pub fn from_slice(contents: &[u8]) -> anyhow::Result<Self> {
        serde_json::from_slice(contents).map_err(Into::into)
    }

    pub fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    #[test]
    fn test_metadata_from_created_at_sets_created_at() {
        let created_at = Utc.with_ymd_and_hms(2026, 5, 24, 1, 2, 3).unwrap();

        let metadata = Metadata::from_created_at(created_at);

        assert_eq!(metadata.created_at, created_at);
        assert_eq!(metadata.archived_at, None);
    }

    #[test]
    fn test_metadata_archived_sets_archived_at() {
        let created_at = Utc.with_ymd_and_hms(2026, 5, 24, 1, 2, 3).unwrap();
        let archived_at = Utc.with_ymd_and_hms(2026, 5, 25, 4, 5, 6).unwrap();
        let metadata = Metadata::from_created_at(created_at);

        let metadata = metadata.archived(archived_at);

        assert_eq!(metadata.created_at, created_at);
        assert_eq!(metadata.archived_at, Some(archived_at));
    }

    #[test]
    fn test_metadata_unarchived_clears_archived_at() {
        let created_at = Utc.with_ymd_and_hms(2026, 5, 24, 1, 2, 3).unwrap();
        let archived_at = Utc.with_ymd_and_hms(2026, 5, 25, 4, 5, 6).unwrap();
        let metadata = Metadata::from_created_at(created_at).archived(archived_at);

        let metadata = metadata.unarchived();

        assert_eq!(metadata.created_at, created_at);
        assert_eq!(metadata.archived_at, None);
    }

    #[test]
    fn test_metadata_json_roundtrip() {
        let created_at = Utc.with_ymd_and_hms(2026, 5, 24, 1, 2, 3).unwrap();
        let archived_at = Utc.with_ymd_and_hms(2026, 5, 25, 4, 5, 6).unwrap();
        let metadata = Metadata::from_created_at(created_at).archived(archived_at);

        let content = metadata.to_vec().unwrap();
        let metadata = Metadata::from_slice(&content).unwrap();

        assert_eq!(metadata.created_at, created_at);
        assert_eq!(metadata.archived_at, Some(archived_at));
    }
}
