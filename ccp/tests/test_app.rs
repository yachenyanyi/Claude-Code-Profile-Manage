use ccp::config::profile::Profile;
use ccp::config::store::Store;
use ccp::shell::generator::Generator;
use ccp::tui::app::{App, AppMode, Focus};
use tempfile::tempdir;

fn setup_app() -> (App, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("profiles.toml");
    let bin_dir = dir.path().join("bin");

    let store = Store::with_path(store_path);
    let generator = Generator::with_dir(bin_dir);
    let app = App::new_with(store, generator).unwrap();
    (app, dir)
}

#[test]
fn test_app_initial_state() {
    let (app, _dir) = setup_app();
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.focus, Focus::List);
    assert!(app.config.profiles.is_empty());
}

#[test]
fn test_app_add_profile() {
    let (mut app, _dir) = setup_app();
    let mut p = Profile::new("test-model".to_string());
    p.vars
        .insert("ANTHROPIC_MODEL".to_string(), "deepseek".to_string());

    app.add_profile(p).unwrap();
    assert_eq!(app.config.profiles.len(), 1);
    assert_eq!(app.config.profiles[0].name, "test-model");
}

#[test]
fn test_app_add_duplicate_name() {
    let (mut app, _dir) = setup_app();
    let p1 = Profile::new("test".to_string());
    let p2 = Profile::new("test".to_string());

    app.add_profile(p1).unwrap();
    // 重名应返回 Err（修复后的行为）
    let result = app.add_profile(p2);
    assert!(result.is_err());
    assert_eq!(app.config.profiles.len(), 1);
}

#[test]
fn test_app_toggle_enabled() {
    let (mut app, _dir) = setup_app();
    let p = Profile::new("test".to_string());
    app.add_profile(p).unwrap();
    assert!(app.config.profiles[0].enabled);

    app.toggle_enabled().unwrap();
    assert!(!app.config.profiles[0].enabled);

    app.toggle_enabled().unwrap();
    assert!(app.config.profiles[0].enabled);
}

#[test]
fn test_app_delete_profile() {
    let (mut app, _dir) = setup_app();
    app.add_profile(Profile::new("a".to_string())).unwrap();
    app.add_profile(Profile::new("b".to_string())).unwrap();
    assert_eq!(app.config.profiles.len(), 2);

    app.selected = 0;
    app.delete_current().unwrap();
    assert_eq!(app.config.profiles.len(), 1);
    assert_eq!(app.config.profiles[0].name, "b");
}

#[test]
fn test_app_selection_bounds() {
    let (mut app, _dir) = setup_app();
    app.add_profile(Profile::new("a".to_string())).unwrap();
    app.add_profile(Profile::new("b".to_string())).unwrap();
    app.add_profile(Profile::new("c".to_string())).unwrap();

    app.selected = 2;
    app.delete_current().unwrap();
    assert!(app.selected < app.config.profiles.len());
}

#[test]
fn test_app_update_profile() {
    let (mut app, _dir) = setup_app();
    app.add_profile(Profile::new("original".to_string())).unwrap();

    let mut updated = Profile::new("updated".to_string());
    updated.enabled = false;
    app.update_profile(0, updated).unwrap();

    assert_eq!(app.config.profiles[0].name, "updated");
    assert!(!app.config.profiles[0].enabled);
}
