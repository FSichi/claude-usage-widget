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
    println!("cargo:rerun-if-changed=build.rs");
}
