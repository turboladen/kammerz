//! Embeds the git commit as KAMMERZ_BUILD_SHA so the startup log and
//! GET /api/health can identify the running build. CARGO_PKG_VERSION alone
//! can't: releases are untagged deploys (`just deploy`), so the crate version
//! rarely changes between builds. `just deploy` compares this SHA against the
//! local HEAD to prove the NEW binary is the one actually serving.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let sha = git(&["rev-parse", "--short=8", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
    println!(
        "cargo:rustc-env=KAMMERZ_BUILD_SHA={sha}{}",
        if dirty { "-dirty" } else { "" }
    );
    // Invalidate when HEAD moves (commit/branch switch). Without this the
    // embedded SHA goes stale in incremental builds. `.git/HEAD` covers
    // checkouts; the logs/HEAD reflog is touched by every commit. If a path
    // doesn't exist (bare-ish worktrees), cargo just reruns the script —
    // cheap and safe.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/logs/HEAD");
}
