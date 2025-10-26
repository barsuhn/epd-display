use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let memory_x = if cfg!(feature = "rp2350") {
        include_str!("memory-rp2350.x")
    } else if cfg!(feature = "rp2040") {
        include_str!("memory-rp2040.x")
    } else {
        panic!("Either rp2040 or rp2350 feature must be enabled");
    };

    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(memory_x.as_bytes())
        .unwrap();

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory-rp2040.x");
    println!("cargo:rerun-if-changed=memory-rp2350.x");
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    if cfg!(feature = "rp2040") {
        println!("cargo:rustc-link-arg-bins=-Tlink-rp.x");
    }
}
