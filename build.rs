use embed_manifest::manifest::DpiAwareness;
use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    // A layered child of the taskbar needs the Windows 8+ compatibility entries
    // this manifest declares.
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest(
            new_manifest("ClaudeUsageWidget").dpi_awareness(DpiAwareness::PerMonitorV2Only),
        )
        .expect("unable to embed manifest");
    }
    // The icon needs a resource compiler, which only the MSVC build has to hand.
    // That is the build the release workflow makes.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("unable to embed icon");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");
}
