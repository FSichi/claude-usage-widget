//! Sets up the mod that feeds the widget, so nobody has to know it exists.
//!
//! The mod ships inside the exe. On a first run, or when a new exe carries a
//! newer mod, it is unpacked and registered through Claude Code's own CLI.

use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use crate::settings::data_dir;

const MARKETPLACE: &str = "claude-usage-widget";
const PLUGIN: &str = "claude-usage-bridge";
const STAMP: &str = "installed-version";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const MANIFEST: &str = include_str!("../mod/.claude-plugin/plugin.json");
const CATALOG: &str = include_str!("../.claude-plugin/marketplace.json");
const FILES: [(&str, &str); 4] = [
    (".claude-plugin/marketplace.json", CATALOG),
    ("mod/.claude-plugin/plugin.json", MANIFEST),
    (
        "mod/hooks/hooks.json",
        include_str!("../mod/hooks/hooks.json"),
    ),
    (
        "mod/hooks/register.ts",
        include_str!("../mod/hooks/register.ts"),
    ),
];

static CLAUDE_MISSING: AtomicBool = AtomicBool::new(false);

/// True once setup found no Claude Code to register the mod with.
pub fn claude_is_missing() -> bool {
    CLAUDE_MISSING.load(Ordering::Relaxed)
}

/// On its own thread: the CLI takes seconds and the UI thread shares input
/// with the taskbar.
pub fn ensure_installed() {
    std::thread::spawn(|| {
        let root = data_dir().join("bridge");
        let Some(version) = bundled_version() else {
            return;
        };
        if fs::read_to_string(root.join(STAMP)).ok().as_deref() == Some(version.as_str()) {
            return;
        }
        if unpack(&root).is_err() {
            return;
        }
        match find_claude() {
            Some(claude) if register(&claude, &root) => {
                let _ = fs::write(root.join(STAMP), version);
            }
            Some(_) => {}
            None => CLAUDE_MISSING.store(true, Ordering::Relaxed),
        }
    });
}

fn bundled_version() -> Option<String> {
    let manifest: serde_json::Value = serde_json::from_str(MANIFEST).ok()?;
    Some(manifest.get("version")?.as_str()?.to_string())
}

fn unpack(root: &Path) -> std::io::Result<()> {
    for (path, text) in FILES {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, text)?;
    }
    Ok(())
}

/// Every step tolerates having run before: a known catalog only refreshes, and
/// a plugin already there updates instead of installing.
fn register(claude: &Path, root: &Path) -> bool {
    let id = format!("{PLUGIN}@{MARKETPLACE}");
    let root = root.to_string_lossy();
    run(claude, &["plugin", "marketplace", "add", &root]);
    run(claude, &["plugin", "marketplace", "update", MARKETPLACE]);
    run(claude, &["plugin", "install", &id, "--scope", "user"])
        || run(claude, &["plugin", "update", &id, "--scope", "user"])
}

fn run(claude: &Path, args: &[&str]) -> bool {
    Command::new(claude)
        .args(args)
        .env_remove("CLAUDECODE")
        .env_remove("CLAUDE_CODE_ENTRYPOINT")
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn find_claude() -> Option<PathBuf> {
    let home = env::var_os("USERPROFILE").map(PathBuf::from);
    let installed = home.map(|home| home.join(".local").join("bin").join("claude.exe"));
    let on_path = env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join("claude.exe"));
    installed
        .into_iter()
        .chain(on_path)
        .find(|path| path.is_file())
        .or_else(newest_desktop_copy)
}

/// The desktop app carries its own Claude Code, for people who never
/// installed the CLI: `claude-code/<version>[/<build>]/claude.exe`.
fn newest_desktop_copy() -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        roots.extend(
            subdirectories(&local.join("Claude-Data"))
                .into_iter()
                .map(|dir| dir.join("claude-code")),
        );
    }
    if let Some(roaming) = env::var_os("APPDATA").map(PathBuf::from) {
        roots.push(roaming.join("Claude").join("claude-code"));
    }
    let copies = roots
        .iter()
        .flat_map(|root| subdirectories(root))
        .flat_map(|version| {
            let builds = subdirectories(&version);
            std::iter::once(version).chain(builds)
        })
        .map(|dir| dir.join("claude.exe"))
        .filter_map(|path| Some((fs::metadata(&path).ok()?.modified().ok()?, path)));
    newest(copies)
}

fn newest(copies: impl Iterator<Item = (SystemTime, PathBuf)>) -> Option<PathBuf> {
    copies
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn subdirectories(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn name_of(text: &str) -> String {
        let value: serde_json::Value = serde_json::from_str(text).unwrap();
        value["name"].as_str().unwrap().to_string()
    }

    #[test]
    fn the_ids_match_the_files_they_install() {
        assert_eq!(name_of(MANIFEST), PLUGIN);
        assert_eq!(name_of(CATALOG), MARKETPLACE);
        let catalog: serde_json::Value = serde_json::from_str(CATALOG).unwrap();
        assert_eq!(catalog["plugins"][0]["name"], PLUGIN);
    }

    #[test]
    fn the_catalog_points_at_the_unpacked_mod() {
        let catalog: serde_json::Value = serde_json::from_str(CATALOG).unwrap();
        let source = catalog["plugins"][0]["source"].as_str().unwrap();
        let folder = source.trim_start_matches("./");
        assert!(FILES.iter().any(|(path, _)| path.starts_with(folder)));
    }

    #[test]
    fn the_stamp_and_the_mod_agree_on_the_version() {
        let version = bundled_version().unwrap();
        let (_, module) = FILES[3];
        assert!(module.contains(&format!("const BRIDGE = '{version}'")));
    }

    #[test]
    fn the_newest_copy_wins() {
        let start = SystemTime::UNIX_EPOCH;
        let older = (start + Duration::from_secs(1), PathBuf::from("old"));
        let newer = (start + Duration::from_secs(2), PathBuf::from("new"));
        let found = newest([newer, older].into_iter());
        assert_eq!(found, Some(PathBuf::from("new")));
        assert_eq!(newest(std::iter::empty()), None);
    }
}
