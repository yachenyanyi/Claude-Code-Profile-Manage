use ccp::config::profile::Profile;
use std::collections::HashMap;

#[test]
fn test_profile_new() {
    let p = Profile::new("deepseek".to_string());
    assert_eq!(p.name, "deepseek");
    assert!(p.enabled);
    assert!(p.vars.is_empty());
    assert!(p.group.is_none());
}

#[test]
fn test_profile_validate_valid() {
    let p = Profile::new("deepseek-v4".to_string());
    assert!(p.validate().is_ok());
}

#[test]
fn test_profile_validate_empty_name() {
    let p = Profile::new("".to_string());
    assert!(p.validate().is_err());
}

#[test]
fn test_profile_validate_invalid_chars() {
    let p = Profile::new("Deep Seek!".to_string());
    assert!(p.validate().is_err());
}

#[test]
fn test_profile_validate_uppercase() {
    let p = Profile::new("DeepSeek".to_string());
    assert!(p.validate().is_err());
}

#[test]
fn test_profile_validate_hyphens_ok() {
    let p = Profile::new("my-deepseek-v4".to_string());
    assert!(p.validate().is_ok());
}

#[test]
fn test_profile_env_var_insert() {
    let mut p = Profile::new("test".to_string());
    p.vars.insert("ANTHROPIC_MODEL".to_string(), "deepseek-v4-flash".to_string());
    assert_eq!(p.vars.get("ANTHROPIC_MODEL").unwrap(), "deepseek-v4-flash");
}