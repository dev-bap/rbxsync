use clap::Parser;
use rbxsync::cli::Cli;

#[test]
fn api_key_from_flag() {
    let cli = Cli::try_parse_from(["rbxsync", "--api-key", "test-key-123", "check"]).unwrap();
    assert_eq!(cli.api_key.as_deref(), Some("test-key-123"));
}

/// Env var tests must run sequentially (shared process state), so they live in
/// a single test function to avoid races with parallel test execution.
#[test]
fn api_key_env_var() {
    // 1. Absent when env var is not set
    unsafe { std::env::remove_var("RBXSYNC_API_KEY") };
    let cli = Cli::try_parse_from(["rbxsync", "check"]).unwrap();
    assert!(cli.api_key.is_none(), "should be None when env var is unset");

    // 2. Picked up from env var
    unsafe { std::env::set_var("RBXSYNC_API_KEY", "env-key-456") };
    let cli = Cli::try_parse_from(["rbxsync", "check"]).unwrap();
    assert_eq!(cli.api_key.as_deref(), Some("env-key-456"));

    // 3. CLI flag overrides env var
    let cli = Cli::try_parse_from(["rbxsync", "--api-key", "flag-key", "check"]).unwrap();
    assert_eq!(cli.api_key.as_deref(), Some("flag-key"));

    // Cleanup
    unsafe { std::env::remove_var("RBXSYNC_API_KEY") };
}
