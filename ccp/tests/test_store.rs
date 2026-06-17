use ccp::config::store::{Config, Store};
use ccp::config::profile::Profile;
use tempfile::tempdir;

#[test]
fn test_store_save_and_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("profiles.toml");
    let store = Store::with_path(path.clone());

    let mut p = Profile::new("test".to_string());
    p.vars.insert(
        "ANTHROPIC_MODEL".to_string(),
        "deepseek-v4".to_string(),
    );

    let config = Config {
        profiles: vec![p],
    };
    store.save(&config).unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.profiles.len(), 1);
    assert_eq!(loaded.profiles[0].name, "test");
    assert_eq!(
        loaded.profiles[0].vars.get("ANTHROPIC_MODEL").unwrap(),
        "deepseek-v4"
    );
}

#[test]
fn test_store_load_empty_file_returns_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.toml");
    std::fs::write(&path, "").unwrap();
    let store = Store::with_path(path);
    let config = store.load().unwrap();
    assert!(config.profiles.is_empty());
}

#[test]
fn test_store_load_missing_file_returns_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.toml");
    let store = Store::with_path(path);
    let config = store.load().unwrap();
    assert!(config.profiles.is_empty());
}

#[test]
fn test_store_default_path() {
    let store = Store::new();
    assert!(store.path().to_string_lossy().contains("ccp/profiles.toml"));
}
