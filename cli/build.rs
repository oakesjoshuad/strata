use std::process::Command;

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);
    let build_id = if dirty {
        format!("{commit}-dirty")
    } else {
        commit
    };
    println!("cargo:rustc-env=STRATA_BUILD_ID={build_id}");
    let package_version = match std::env::var("CARGO_PKG_VERSION") {
        Ok(value) => value,
        Err(_) => "0.1.0".to_string(),
    };
    println!("cargo:rustc-env=STRATA_VERSION={package_version}+{build_id}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
}
