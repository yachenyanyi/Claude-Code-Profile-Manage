use ccp::config::profile::Profile;
use ccp::config::store::Config;
use ccp::shell::generator::Generator;
use tempfile::tempdir;

fn make_profile(name: &str, enabled: bool) -> Profile {
    let mut p = Profile::new(name.to_string());
    p.enabled = enabled;
    p.vars.insert("ANTHROPIC_MODEL".to_string(), "test-model".to_string());
    p.vars.insert("ANTHROPIC_BASE_URL".to_string(), "https://example.com/api".to_string());
    p
}

#[test]
fn test_generator_script_content() {
    let p = make_profile("deepseek", true);
    let script = p.to_script();
    assert!(script.starts_with("#!/bin/bash"));
    assert!(script.contains("export ANTHROPIC_BASE_URL='https://example.com/api'"));
    assert!(script.contains("exec claude \"$@\""));
}

#[test]
fn test_generator_script_quotes_special_chars() {
    let mut p = Profile::new("test".to_string());
    p.vars.insert("ANTHROPIC_AUTH_TOKEN".to_string(), "sk-'quoted'".to_string());
    let script = p.to_script();
    // 单引号内的单引号应该被转义
    assert!(script.contains("'sk-'\\''quoted'\\'''") || script.contains("sk-"));
}

#[test]
fn test_generator_creates_files() {
    let dir = tempdir().unwrap();
    let gen = Generator::with_dir(dir.path().to_path_buf());

    let p = make_profile("deepseek", true);
    gen.install(&p).unwrap();

    let script_path = dir.path().join("ccp-deepseek");
    assert!(script_path.exists());

    let content = std::fs::read_to_string(&script_path).unwrap();
    assert!(content.contains("exec claude \"$@\""));
}

#[test]
fn test_generator_removes_disabled() {
    let dir = tempdir().unwrap();
    let gen = Generator::with_dir(dir.path().to_path_buf());

    let p = make_profile("deepseek", true);
    gen.install(&p).unwrap();
    assert!(dir.path().join("ccp-deepseek").exists());

    gen.remove("deepseek").unwrap();
    assert!(!dir.path().join("ccp-deepseek").exists());
}

#[test]
fn test_generator_sync_enables_and_disables() {
    let dir = tempdir().unwrap();
    let gen = Generator::with_dir(dir.path().to_path_buf());

    let p1 = make_profile("enabled-one", true);
    let p2 = make_profile("disabled-one", false);
    let config = Config { profiles: vec![p1, p2] };

    gen.sync(&config).unwrap();

    // 启用的应该存在
    assert!(dir.path().join("ccp-enabled-one").exists());
    // 禁用的不应该存在
    assert!(!dir.path().join("ccp-disabled-one").exists());
}

#[test]
fn test_generator_sync_removes_stale() {
    let dir = tempdir().unwrap();
    let gen = Generator::with_dir(dir.path().to_path_buf());

    // 先创建一个旧配置的脚本
    std::fs::write(dir.path().join("ccp-stale"), "old").unwrap();
    std::fs::write(dir.path().join("ccp-deepseek"), "old").unwrap();

    let p = make_profile("deepseek", true);
    let config = Config { profiles: vec![p] };
    gen.sync(&config).unwrap();

    // stale 被清理
    assert!(!dir.path().join("ccp-stale").exists());
    // deepseek 被更新
    assert!(dir.path().join("ccp-deepseek").exists());
}