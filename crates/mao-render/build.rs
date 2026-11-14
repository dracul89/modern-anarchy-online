//! `intel_tex_2` ships C++ objects. On Linux the final link needs the C++ runtime too.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
