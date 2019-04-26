extern crate bindgen;
extern crate cc;

fn main() {
    let mut config = cc::Build::new();
    config.file("misc.h");
    config.file("misc.c");
    config.compile("libtools.a");

    println!("rerun-if-changed=build.rs");
    println!("rerun-if-changed=misc.h");
    println!("rerun-if-changed=misc.c");

    println!("cargo:rustc-link-search=/usr/local/opt/openssl/lib");
    println!("cargo:rustc-link-search=/usr/local/lib");

    println!("cargo:rustc-link-lib=static=etpan");
    println!("cargo:rustc-link-lib=dylib=iconv");
    println!("cargo:rustc-link-lib=dylib=sasl2");
    println!("cargo:rustc-link-lib=dylib=z");

    println!("cargo:rustc-link-lib=dylib=sqlite3");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=crypto");
    println!("cargo:rustc-link-lib=dylib=tools");

    if std::env::var("TARGET").unwrap().contains("-apple") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreServices");
        println!("cargo:rustc-link-lib=framework=Security");
    }
}
