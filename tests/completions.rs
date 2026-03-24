//! Shell completions integration tests
//!
//! Tests that `jed completions <shell>` generates valid completion scripts
//! for bash, zsh, fish, powershell, and elvish.

use std::process::Command;

fn get_jed_binary() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    format!("{manifest_dir}/target/release/jed")
}

#[test]
fn test_bash_completion() {
    let output = Command::new(get_jed_binary())
        .args(["completions", "bash"])
        .output()
        .expect("Failed to execute jed");

    assert!(output.status.success(), "completions bash should succeed");

    let script = String::from_utf8_lossy(&output.stdout);

    // Check for essential bash completion elements
    assert!(script.contains("_jed()"), "Should define _jed function");
    assert!(script.contains("COMPREPLY"), "Should use COMPREPLY");
    assert!(
        script.contains("complete -F _jed"),
        "Should register completion"
    );

    // Check for some commands
    assert!(script.contains("get"), "Should include 'get' command");
    assert!(script.contains("set"), "Should include 'set' command");
    assert!(script.contains("fix"), "Should include 'fix' command");
}

#[test]
fn test_zsh_completion() {
    let output = Command::new(get_jed_binary())
        .args(["completions", "zsh"])
        .output()
        .expect("Failed to execute jed");

    assert!(output.status.success(), "completions zsh should succeed");

    let script = String::from_utf8_lossy(&output.stdout);

    // Check for essential zsh completion elements
    assert!(
        script.contains("#compdef jed"),
        "Should start with #compdef jed"
    );
    assert!(script.contains("_jed()"), "Should define _jed function");

    // Check for some commands
    assert!(script.contains("get)"), "Should include 'get' command");
    assert!(script.contains("set)"), "Should include 'set' command");
}

#[test]
fn test_fish_completion() {
    let output = Command::new(get_jed_binary())
        .args(["completions", "fish"])
        .output()
        .expect("Failed to execute jed");

    assert!(output.status.success(), "completions fish should succeed");

    let script = String::from_utf8_lossy(&output.stdout);

    // Check for essential fish completion elements
    assert!(
        script.contains("complete -c jed"),
        "Should use 'complete -c jed'"
    );

    // Check for global options (fish uses -l for --long-option)
    assert!(
        script.contains("-l json") || script.contains("--json"),
        "Should include json option"
    );
    assert!(
        script.contains("-l quiet") || script.contains("--quiet"),
        "Should include quiet option"
    );

    // Check for some commands
    assert!(
        script.contains("-a \"get\""),
        "Should include 'get' command"
    );
    assert!(
        script.contains("-a \"set\""),
        "Should include 'set' command"
    );
}

#[test]
fn test_powershell_completion() {
    let output = Command::new(get_jed_binary())
        .args(["completions", "powershell"])
        .output()
        .expect("Failed to execute jed");

    assert!(
        output.status.success(),
        "completions powershell should succeed"
    );

    let script = String::from_utf8_lossy(&output.stdout);

    // Check for essential powershell completion elements
    assert!(
        script.contains("Register-ArgumentCompleter"),
        "Should use Register-ArgumentCompleter"
    );
    assert!(script.contains("jed"), "Should reference jed command");
}

#[test]
fn test_elvish_completion() {
    let output = Command::new(get_jed_binary())
        .args(["completions", "elvish"])
        .output()
        .expect("Failed to execute jed");

    assert!(output.status.success(), "completions elvish should succeed");

    let script = String::from_utf8_lossy(&output.stdout);

    // Check for essential elvish completion elements
    assert!(
        script.contains("set edit:completion:arg-completer[jed]"),
        "Should set elvish completer"
    );
}

#[test]
fn test_invalid_shell() {
    let output = Command::new(get_jed_binary())
        .args(["completions", "invalid-shell"])
        .output()
        .expect("Failed to execute jed");

    // Should fail with error
    assert!(!output.status.success(), "Invalid shell should fail");
}

#[test]
fn test_completion_includes_all_commands() {
    let output = Command::new(get_jed_binary())
        .args(["completions", "bash"])
        .output()
        .expect("Failed to execute jed");

    assert!(output.status.success());
    let script = String::from_utf8_lossy(&output.stdout);

    // All major commands should be present
    let commands = [
        "get",
        "set",
        "del",
        "add",
        "mv",
        "patch",
        "keys",
        "len",
        "type",
        "exists",
        "schema",
        "check",
        "fmt",
        "fix",
        "minify",
        "diff",
        "tree",
        "query",
        "validate",
        "convert",
        "commands",
        "explain",
        "completions",
    ];

    for cmd in commands {
        assert!(
            script.contains(cmd),
            "Completion should include '{cmd}' command"
        );
    }
}
