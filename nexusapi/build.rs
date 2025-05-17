use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::exit;

extern crate version_check as rustc;

use convert_case::Casing;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct CargoLock {
    package: Vec<PackageInfo>,
}

fn main() {
    let cargo_lock_raw = include_str!("../Cargo.lock");
    let cargo_lock: CargoLock = toml::from_str(cargo_lock_raw).unwrap();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("user_agent.rs");

    let mut vsn_info: HashMap<String, String> = HashMap::new();

    for package in cargo_lock.package {
        let name = package.name;
        let vsn = package.version;

        vsn_info
            .entry(name)
            .and_modify(|v| {
                v.push_str(", ");
                v.push_str(vsn.as_str());
            })
            .or_insert(vsn);
    }

    let package_name = env::var_os("CARGO_PKG_NAME")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let package_vsn = env::var_os("CARGO_PKG_VERSION")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let platform_os = env::var_os("CARGO_CFG_TARGET_OS")
        .unwrap()
        .to_str()
        .unwrap()
        .to_case(convert_case::Case::Title);
    let platform_arch = env::var_os("CARGO_CFG_TARGET_ARCH")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let profile_suffix = match env::var_os("PROFILE").unwrap().to_str().unwrap() {
        "release" => "",
        "debug" => "+debug",
        _ => "+unknown",
    };

    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=src/lib.rs");
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=Cargo.lock");

    let rust_vsn;
    if let Some((rust_maj, rust_min, rust_patch)) = rustc::triple() {
        rust_vsn = format!("{rust_maj}.{rust_min}.{rust_patch}");
    } else {
        println!("cargo::error=Unable to determine Rust version");
        exit(1);
    };
    let reqwest_vsn = vsn_info.get("reqwest").unwrap();

    let user_agent = format!(
        "{package_name}/{package_vsn}{profile_suffix} ({platform_os} {platform_arch}) Rust/{rust_vsn} reqwest/{reqwest_vsn}"
    );

    let code = format!("const USER_AGENT: &str = \"{user_agent}\";");

    fs::write(&dest_path, code).unwrap();
}
