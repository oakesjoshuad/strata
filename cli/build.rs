fn main() {
    let build_id = kernel::build::git_build_id();
    println!("cargo:rustc-env=STRATA_BUILD_ID={build_id}");
    let package_version = match std::env::var("CARGO_PKG_VERSION") {
        Ok(value) => value,
        Err(_) => "0.1.0".to_string(),
    };
    println!("cargo:rustc-env=STRATA_VERSION={package_version}+{build_id}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
}
