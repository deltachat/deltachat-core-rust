extern crate bindgen;
extern crate cc;

fn main() {
    let mut config = cc::Build::new();
    config.file("misc.h");
    config.file("misc.c");
    config.compile("libtools.a");

    println!("cargo:rustc-link-search=native=/usr/local/opt/openssl/lib");

    println!("cargo:rustc-link-lib=dylib=sasl2");
    println!("cargo:rustc-link-lib=dylib=ssl");
    println!("cargo:rustc-link-lib=dylib=sqlite3");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=rpgp");
    println!("cargo:rustc-link-lib=dylib=etpan");
    println!("cargo:rustc-link-lib=dylib=iconv");
    println!("cargo:rustc-link-lib=dylib=crypto");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=tools");

    if std::env::var("TARGET").unwrap().contains("-apple") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreServices");
        println!("cargo:rustc-link-lib=framework=Security");
    }
}
