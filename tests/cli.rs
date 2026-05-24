use std::path::Path;
use std::process::{Command, Output};

use chrono::{DateTime, Duration, Utc};
use tempfile::TempDir;

const TIMESTAMP_LEEWAY: Duration = Duration::seconds(10);

fn run_gist(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gist"))
        .args(args)
        .env("TZ", "Asia/Tokyo")
        .env_remove("GIST_ROOT")
        .output()
        .unwrap()
}

fn run_gist_with_root(root: &Path, args: &[&str]) -> Output {
    let mut full_args = vec!["--root", root.to_str().unwrap()];
    full_args.extend_from_slice(args);
    run_gist(&full_args)
}

fn run_gist_with_env_root(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gist"))
        .args(args)
        .env("TZ", "Asia/Tokyo")
        .env("GIST_ROOT", root)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn write_metadata(directory: &Path, created_at: &str, archived_at: Option<&str>) {
    std::fs::create_dir(directory).unwrap();
    let content = serde_json::to_vec_pretty(&serde_json::json!({
        "created_at": created_at,
        "archived_at": archived_at,
    }))
    .unwrap();
    std::fs::write(directory.join(".gist.json"), content).unwrap();
}

fn metadata(directory: &Path) -> serde_json::Value {
    let content = std::fs::read(directory.join(".gist.json")).unwrap();
    serde_json::from_slice(&content).unwrap()
}

fn assert_datetime_between(value: &serde_json::Value, before: DateTime<Utc>, after: DateTime<Utc>) {
    let datetime = value.as_str().unwrap().parse::<DateTime<Utc>>().unwrap();
    assert!(datetime >= before - TIMESTAMP_LEEWAY);
    assert!(datetime <= after + TIMESTAMP_LEEWAY);
}

#[test]
fn test_root_uses_gist_root_env() {
    let root = TempDir::new().unwrap();

    let output = run_gist_with_env_root(root.path(), &["root"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), format!("{}\n", root.path().canonicalize().unwrap().display()));
}

#[test]
fn test_root_prints_canonical_root() {
    let root = TempDir::new().unwrap();

    let output = run_gist_with_root(root.path(), &["root"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), format!("{}\n", root.path().canonicalize().unwrap().display()));
}

#[test]
fn test_root_fails_for_missing_root() {
    let root = TempDir::new().unwrap();
    let missing = root.path().join("missing");

    let output = run_gist(&["--root", missing.to_str().unwrap(), "root"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("cannot resolve gist root"));
}

#[test]
fn test_root_fails_for_file_root() {
    let root = TempDir::new().unwrap();
    let file = root.path().join("file");
    std::fs::write(&file, b"not a directory").unwrap();

    let output = run_gist(&["--root", file.to_str().unwrap(), "root"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("is not a directory"));
}

#[test]
fn test_create_creates_files() {
    let root = TempDir::new().unwrap();

    let before = Utc::now();
    let output = run_gist_with_root(root.path(), &["create", "alpha"]);
    let after = Utc::now();

    assert!(output.status.success(), "{}", stderr(&output));
    let directory = root.path().join("alpha");
    assert!(directory.is_dir());
    assert_eq!(std::fs::read(directory.join(".vscode/settings.json")).unwrap(), b"{\n}");

    let metadata = metadata(&directory);
    assert_datetime_between(&metadata["created_at"], before, after);
    assert!(metadata["archived_at"].is_null());
}

#[test]
fn test_create_fails_for_existing_gist() {
    let root = TempDir::new().unwrap();
    let output = run_gist_with_root(root.path(), &["create", "alpha"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let output = run_gist_with_root(root.path(), &["create", "alpha"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("already exists"));
}

#[test]
fn test_create_continues_after_partial_failure() {
    let root = TempDir::new().unwrap();
    let output = run_gist_with_root(root.path(), &["create", "alpha"]);
    assert!(output.status.success(), "{}", stderr(&output));

    let output = run_gist_with_root(root.path(), &["create", "alpha", "beta"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("some gists failed"));
    assert!(root.path().join("beta").is_dir());
    assert!(metadata(&root.path().join("beta"))["archived_at"].is_null());
}

#[test]
fn test_list_prints_sorted_gists() {
    let root = TempDir::new().unwrap();
    write_metadata(&root.path().join("alpha"), "2026-05-24T01:00:00Z", None);
    write_metadata(&root.path().join("beta"), "2026-05-24T02:00:00Z", None);
    write_metadata(&root.path().join("gamma"), "2026-05-24T01:00:00Z", None);
    write_metadata(
        &root.path().join("archived"),
        "2026-05-25T01:00:00Z",
        Some("2026-05-26T01:00:00Z"),
    );
    std::fs::write(root.path().join("note.txt"), b"not a gist").unwrap();
    std::fs::create_dir(root.path().join("plain-directory")).unwrap();

    let output = run_gist_with_root(root.path(), &["list"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "\
2026-05-24 11:00 | beta
2026-05-24 10:00 | alpha
2026-05-24 10:00 | gamma
2026-05-25 10:00 | archived (archived)
"
    );
}

#[test]
fn test_list_skips_broken_metadata() {
    let root = TempDir::new().unwrap();
    write_metadata(&root.path().join("alpha"), "2026-05-24T01:00:00Z", None);
    std::fs::create_dir(root.path().join("broken")).unwrap();
    std::fs::write(root.path().join("broken/.gist.json"), b"not json").unwrap();

    let output = run_gist_with_root(root.path(), &["list"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "2026-05-24 10:00 | alpha\n");
    assert!(stderr(&output).contains("cannot parse"));
}

#[test]
fn test_archive_sets_archived_at() {
    let root = TempDir::new().unwrap();
    let directory = root.path().join("alpha");
    write_metadata(&directory, "2026-05-24T01:00:00Z", None);

    let before = Utc::now();
    let output = run_gist_with_root(root.path(), &["archive", "alpha"]);
    let after = Utc::now();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_datetime_between(&metadata(&directory)["archived_at"], before, after);
}

#[test]
fn test_archive_fails_for_missing_gist() {
    let root = TempDir::new().unwrap();

    let output = run_gist_with_root(root.path(), &["archive", "missing"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("cannot read existing metadata"));
}

#[test]
fn test_archive_continues_after_partial_failure() {
    let root = TempDir::new().unwrap();
    let alpha = root.path().join("alpha");
    let beta = root.path().join("beta");
    write_metadata(&alpha, "2026-05-24T01:00:00Z", None);
    write_metadata(&beta, "2026-05-24T02:00:00Z", None);

    let before = Utc::now();
    let output = run_gist_with_root(root.path(), &["archive", "alpha", "missing", "beta"]);
    let after = Utc::now();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("some gists failed"));
    assert_datetime_between(&metadata(&alpha)["archived_at"], before, after);
    assert_datetime_between(&metadata(&beta)["archived_at"], before, after);
}

#[test]
fn test_unarchive_clears_archived_at() {
    let root = TempDir::new().unwrap();
    let directory = root.path().join("alpha");
    write_metadata(&directory, "2026-05-24T01:00:00Z", Some("2026-05-25T01:00:00Z"));

    let output = run_gist_with_root(root.path(), &["unarchive", "alpha"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(metadata(&directory)["archived_at"].is_null());
}

#[test]
fn test_unarchive_fails_for_missing_gist() {
    let root = TempDir::new().unwrap();

    let output = run_gist_with_root(root.path(), &["unarchive", "missing"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("cannot read existing metadata"));
}

#[test]
fn test_unarchive_continues_after_partial_failure() {
    let root = TempDir::new().unwrap();
    let alpha = root.path().join("alpha");
    let beta = root.path().join("beta");
    write_metadata(&alpha, "2026-05-24T01:00:00Z", Some("2026-05-25T01:00:00Z"));
    write_metadata(&beta, "2026-05-24T02:00:00Z", Some("2026-05-25T02:00:00Z"));

    let output = run_gist_with_root(root.path(), &["unarchive", "alpha", "missing", "beta"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("some gists failed"));
    assert!(metadata(&alpha)["archived_at"].is_null());
    assert!(metadata(&beta)["archived_at"].is_null());
}
