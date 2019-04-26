extern crate cc;

use std::env;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut config = cc::Build::new();
    config.file("misc.h");
    config.file("misc.c");
    config.compile("libtools.a");

    println!("rerun-if-changed=build.rs");
    println!("rerun-if-changed=misc.h");
    println!("rerun-if-changed=misc.c");

    println!("cargo:rustc-link-search=/usr/local/lib");

    println!("cargo:rustc-link-lib=dylib=sasl2");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=tools");

    if std::env::var("TARGET").unwrap().contains("-apple") {
        println!("cargo:rustc-link-search=/usr/local/opt/openssl/lib");
        println!("cargo:rustc-link-lib=static=etpan");
        println!("cargo:rustc-link-lib=dylib=iconv");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreServices");
        println!("cargo:rustc-link-lib=framework=Security");
    } else if std::env::var("TARGET").unwrap().contains("linux") {
        println!("cargo:rustc-link-lib=dylib=etpan");
    }

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let cfg = cbindgen::Config::from_root_or_default(std::path::Path::new(&crate_dir));
    let c = cbindgen::Builder::new()
        .with_config(cfg)
        .with_crate(crate_dir)
        .with_header(format!("/* deltachat Header Version {} */", VERSION))
        // .with_language(cbindgen::Language::C)
        .generate();

    // This is needed to ensure we don't panic if there are errors in the crates code
    // but rather just tell the rest of the system we can't proceed.
    match c {
        Ok(res) => {
            res.write_to_file("deltachat.h");
        }
        Err(err) => {
            eprintln!("unable to generate bindings: {:#?}", err);
            std::process::exit(1);
        }
    }
}
