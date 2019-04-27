extern crate cc;

use std::env;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn link_dylib(lib: &str) {
    println!("cargo:rustc-link-lib=dylib={}", lib);
}

fn link_static(lib: &str) {
    println!("cargo:rustc-link-lib=static={}", lib);
}

fn link_framework(fw: &str) {
    println!("cargo:rustc-link-lib=framework={}", fw);
}

fn add_search_path(p: &str) {
    println!("cargo:rustc-link-search={}", p);
}

fn build_tools() {
    let mut config = cc::Build::new();
    config.file("misc.h");
    config.file("misc.c");
    config.compile("libtools.a");

    println!("rerun-if-changed=build.rs");
    println!("rerun-if-changed=misc.h");
    println!("rerun-if-changed=misc.c");
}

fn generate_bindings() {
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

fn main() {
    build_tools();

    add_search_path("/usr/local/lib");

    link_dylib("sasl2");
    link_dylib("z");
    link_dylib("pthread");
    link_dylib("tools");

    if std::env::var("TARGET").unwrap().contains("-apple") {
        link_static("etpan");
        link_dylib("iconv");

        link_framework("CoreFoundation");
        link_framework("CoreServices");
        link_framework("Security");
    } else if std::env::var("TARGET").unwrap().contains("linux") {
        link_dylib("etpan");
    }

    generate_bindings();
}
