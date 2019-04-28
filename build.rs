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
    add_search_path("./include/include");
    add_search_path("./include/libs");

    let target = std::env::var("TARGET").unwrap();
    if target.contains("-apple") || target.contains("-darwin") {
        link_static("etpan");
        link_dylib("iconv");

        link_framework("CoreFoundation");
        link_framework("CoreServices");
        link_framework("Security");

        link_dylib("sasl2");
        link_dylib("z");
        link_dylib("pthread");
    } else if target.contains("-android") {
        add_search_path("./include/cyrus-sasl-android-4/include");
        add_search_path("./include/openssl-android-3/include");
        add_search_path("./include/iconv-android-1/include");

        let arch = if target.contains("x86") {
            "x86"
        } else if target.contains("64") {
            "arm64-v8a"
        } else if target.contains("v7") {
            "armeabi-v7a"
        } else {
            "armeabi"
        };

        add_search_path(&format!("./include/libs/{}", arch));
        add_search_path(&format!("./include/iconv-android-1/libs/{}", arch));
        add_search_path(&format!("./include/openssl-android-3/libs/{}", arch));
        add_search_path(&format!("./include/cyrus-sasl-android-4/libs/{}", arch));

        // dependencies for libetpan
        link_static("crypto");
        link_static("sasl2");
        link_static("iconv");
        link_static("ssl");
        link_dylib("z");

        // libetpan iteself
        link_static("etpan");
    } else if target.contains("-linux") {
        link_dylib("etpan");
        link_dylib("sasl2");
        link_dylib("z");
        link_dylib("pthread");
    } else {
        panic!("unsupported target");
    }

    // local tools
    link_static("tools");
}
