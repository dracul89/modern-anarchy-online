//! The Windows build embeds the client's icon (`assets/icon/mao.ico`: the original GUI's steel emblem on slate) as the
//! executable's icon resource, so Explorer and the taskbar show it. Other targets need nothing here; the window's own icon
//! comes from `src/icon.rs` at run time.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/icon/mao.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icon/mao.ico");
        resource.set("FileDescription", "Modern Anarchy Online");
        resource.set("ProductName", "Modern Anarchy Online");
        if let Err(e) = resource.compile() {
            // (a missing resource compiler leaves the executable without its icon, not without a build)
            println!("cargo:warning=the icon resource was not embedded: {e}");
        }
    }
}
