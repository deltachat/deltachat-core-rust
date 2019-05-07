extern crate cc;

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
    config.file("misc.c").compile("libtools.a");

    println!("rerun-if-changed=build.rs");
    println!("rerun-if-changed=misc.h");
    println!("rerun-if-changed=misc.c");
}

fn main() {
    build_tools();

    add_search_path("/usr/local/lib");

    let target = std::env::var("TARGET").unwrap();
    if target.contains("-apple") || target.contains("-darwin") {
        link_framework("CoreFoundation");
        link_framework("CoreServices");
        link_framework("Security");

        link_dylib("pthread");
    } else if target.contains("-android") {
    } else if target.contains("-linux") {
        link_dylib("pthread");
    } else {
        panic!("unsupported target");
    }

    // local tools
    link_static("tools");
}
