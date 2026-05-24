use std::{
    fs::DirEntry,
    path::{Component, Path, PathBuf},
};

use anyhow::Context;
use chrono::{DateTime, Utc};

use crate::metadata::Metadata;

pub struct Repository {
    root: PathBuf,
}

pub struct Gist {
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
}

impl Repository {
    fn directory(&self, name: &str) -> anyhow::Result<PathBuf> {
        if name.contains(char::is_control) {
            anyhow::bail!("gist name {name:?} contains control characters");
        }
        let path = Path::new(name);
        let mut components = path.components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(c)), None) if !c.is_empty() => Ok(self.root.join(c)),
            _ => anyhow::bail!("gist name {name:?} is not a valid directory name"),
        }
    }

    fn read_metadata(path: impl AsRef<Path>) -> anyhow::Result<Metadata> {
        let path = path.as_ref();
        let content = std::fs::read(path).with_context(|| format!("cannot read {path:?}"))?;
        Metadata::from_slice(&content).with_context(|| format!("cannot parse {path:?}"))
    }

    fn write_metadata(path: impl AsRef<Path>, metadata: &Metadata) -> anyhow::Result<()> {
        let path = path.as_ref();
        let content = metadata.to_vec().expect("cannot serialize metadata");
        let temp_path = path.with_added_extension("tmp");
        std::fs::write(&temp_path, content)
            .with_context(|| format!("cannot write {temp_path:?}"))?;
        std::fs::rename(&temp_path, path)
            .with_context(|| format!("cannot rename temporary file {temp_path:?} to {path:?}"))
    }

    fn gist_from_entry(entry: DirEntry) -> anyhow::Result<Option<Gist>> {
        let file_type = entry
            .file_type()
            .with_context(|| format!("cannot read file type of {:?}", entry.path()))?;
        if !file_type.is_dir() {
            return Ok(None);
        }

        let path = entry.path().join(".gist.json");
        if !path.exists() {
            return Ok(None);
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        let metadata = Self::read_metadata(&path)?;
        Ok(Some(Gist {
            name,
            created_at: metadata.created_at,
            archived: metadata.archived_at.is_some(),
        }))
    }
}

impl Repository {
    pub fn open(root: &Path) -> anyhow::Result<Self> {
        let root = std::fs::canonicalize(root)
            .with_context(|| format!("cannot resolve gist root {root:?}"))?;
        if !root.is_dir() {
            anyhow::bail!("gist root {root:?} is not a directory");
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create(&self, name: &str, created_at: DateTime<Utc>) -> anyhow::Result<()> {
        let directory = self.directory(name)?;
        if directory.exists() {
            anyhow::bail!("gist {name} already exists");
        }

        std::fs::create_dir(&directory)
            .with_context(|| format!("cannot create directory {directory:?}"))?;

        std::fs::create_dir(directory.join(".vscode"))
            .with_context(|| format!("cannot create '.vscode' directory under {directory:?}"))?;
        std::fs::write(directory.join(".vscode/settings.json"), b"{\n}").with_context(|| {
            format!("cannot create '.vscode/settings.json' under {directory:?}")
        })?;

        let metadata = Metadata::from_created_at(created_at);
        Self::write_metadata(directory.join(".gist.json"), &metadata)
            .with_context(|| format!("cannot write '.gist.json' under {directory:?}"))
    }

    pub fn list(&self) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Gist>>> {
        let mut entries = std::fs::read_dir(&self.root)
            .with_context(|| format!("cannot read gist root {:?}", self.root))?;

        Ok(std::iter::from_fn(move || {
            for entry in entries.by_ref() {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => return Some(Err(err).context("cannot read directory entry")),
                };
                match Self::gist_from_entry(entry) {
                    Ok(Some(gist)) => return Some(Ok(gist)),
                    Ok(None) => continue,
                    Err(err) => return Some(Err(err)),
                }
            }
            None
        }))
    }

    pub fn archive(&self, name: &str, archived_at: DateTime<Utc>) -> anyhow::Result<()> {
        let directory = self.directory(name)?;
        let path = directory.join(".gist.json");
        let metadata = Self::read_metadata(&path)
            .with_context(|| format!("cannot read existing metadata in {path:?}"))?;
        let metadata = metadata.archived(archived_at);
        Self::write_metadata(&path, &metadata)
            .with_context(|| format!("cannot write new metadata to {path:?}"))
    }

    pub fn unarchive(&self, name: &str) -> anyhow::Result<()> {
        let directory = self.directory(name)?;
        let path = directory.join(".gist.json");
        let metadata = Self::read_metadata(&path)
            .with_context(|| format!("cannot read existing metadata in {path:?}"))?;
        let metadata = metadata.unarchived();
        Self::write_metadata(&path, &metadata)
            .with_context(|| format!("cannot write new metadata to {path:?}"))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    #[rstest]
    #[case::empty("")]
    #[case::nested("alpha/beta")]
    #[case::parent("..")]
    #[case::current(".")]
    #[case::newline("alpha\nbeta")]
    fn test_create_rejects_invalid_name(#[case] name: &str) {
        let root = TempDir::new().unwrap();
        let repository = Repository::open(root.path()).unwrap();
        let created_at = Utc.with_ymd_and_hms(2026, 5, 24, 1, 2, 3).unwrap();

        let result = repository.create(name, created_at);

        assert!(result.is_err());
    }
}
