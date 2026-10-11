// Path: crates/im_agent/src/repos/git_ignore/rules_tests.rs
// Description: Unit tests for Git ignore rule precedence, directory patterns, and negation

use super::rules::{GitIgnoreRules, IgnoreSources};

fn rules(per_directory: &[(&str, &str)], global: &[&str]) -> GitIgnoreRules {
    let sources = IgnoreSources {
        case_insensitive: false,
        per_directory: per_directory
            .iter()
            .map(|(dir, contents)| (dir.to_string(), contents.to_string()))
            .collect(),
        global: global.iter().map(|contents| contents.to_string()).collect(),
    };
    GitIgnoreRules::build(&sources).expect("rules compile")
}

#[test]
fn rooted_directory_pattern_ignores_everything_beneath() {
    let rules = rules(&[("", "/Saved/\nIntermediate/\n")], &[]);
    assert!(rules.is_ignored("Saved/Logs/Game.log", false));
    assert!(rules.is_ignored("Saved/Config/Windows/Game.ini", false));
    assert!(rules.is_ignored("Plugins/Foo/Intermediate/Build/x.cpp", false));
    assert!(
        !rules.is_ignored("Plugins/Foo/Saved/x.ini", false),
        "rooted"
    );
    assert!(!rules.is_ignored("Source/Saved.cpp", false));
    assert!(!rules.is_ignored("Saved", false), "dir pattern needs a dir");
    assert!(rules.is_ignored("Saved", true));
}

#[test]
fn nested_gitignore_applies_relative_to_its_directory() {
    let rules = rules(&[("", "*.tmp\n"), ("Source/Game", "/Generated/\n")], &[]);
    assert!(rules.is_ignored("Source/Game/Generated/a.h", false));
    assert!(!rules.is_ignored("Generated/a.h", false));
    assert!(!rules.is_ignored("Source/Generated/a.h", false));
    assert!(rules.is_ignored("Source/Game/x.tmp", false));
}

#[test]
fn negation_and_deeper_files_override_shallower() {
    let rules = rules(&[("", "*.log\n!keep.log\n"), ("docs", "!*.log\n")], &[]);
    assert!(rules.is_ignored("a/run.log", false));
    assert!(!rules.is_ignored("a/keep.log", false));
    assert!(!rules.is_ignored("docs/run.log", false), "deeper file wins");
}

#[test]
fn excluded_parent_cannot_be_reincluded_from_inside() {
    let rules = rules(
        &[("", "build/\n!build/keep.txt\n"), ("build", "!x.txt\n")],
        &[],
    );
    assert!(rules.is_ignored("build/keep.txt", false));
    assert!(rules.is_ignored("build/x.txt", false));
}

#[test]
fn per_directory_rules_outrank_global_excludes() {
    let rules = rules(&[("", "!local.cfg\n")], &["*.cfg\n", "*.bak\n"]);
    assert!(rules.is_ignored("a/b.cfg", false));
    assert!(!rules.is_ignored("local.cfg", false));
    assert!(rules.is_ignored("x.bak", false));
}
