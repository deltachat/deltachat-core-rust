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
    config.file("misc.h");
    config.file("misc.c");
    config.compile("libtools.a");

    println!("rerun-if-changed=build.rs");
    println!("rerun-if-changed=misc.h");
    println!("rerun-if-changed=misc.c");
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
}
