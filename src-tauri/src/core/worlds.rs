//! World saves.
//!
//! A world is a set of files named after an 8-hex-digit container id in the save folder: the
//! world itself (`3ad85aea`), rolling copies the game keeps (`3ad85aea-1`, `-2` …) and an index
//! (`3ad85aea-index`, JSON: `{"latest": n, "time": <unix s>, "deleted": false}`) telling which copy
//! is current – 0 is the base file, n is `-n`. A dedicated server always uses `3ad85aea`
//! (verified: "[savedata] Start 'Open Container' on container 3ad85aea"). Characters are stored on
//! the players' PCs, not on the server.

use super::backup::quarantine;
use crate::error::{AppError, AppResult};
use crate::models::*;
use regex::Regex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::SystemTime;

static FILE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9a-fA-F]{8})(?:-(\d+|index))?$").unwrap());

fn ms(t: SystemTime) -> i64 {
    chrono::DateTime::<chrono::Utc>::from(t).timestamp_millis()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSlot {
    pub file: String,
    /// 0 = the base file, n = `<id>-n`.
    pub slot: u32,
    pub size: u64,
    pub modified: i64,
    /// The copy the index points at – the one the server loads.
    pub current: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub id: String,
    /// The container a dedicated server loads (`3ad85aea`).
    pub is_server_world: bool,
    pub slots: Vec<WorldSlot>,
    /// Index: current slot, and when it was written (ms).
    pub latest: Option<u32>,
    pub saved_at: Option<i64>,
    pub deleted: bool,
    pub size: u64,
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct Index {
    latest: u32,
    time: Option<i64>,
    deleted: bool,
}

fn read_index(path: &Path) -> Option<Index> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Some(Index {
        latest: v["latest"].as_u64().unwrap_or(0) as u32,
        time: v["time"].as_i64().map(|s| s * 1000),
        deleted: v["deleted"].as_bool().unwrap_or(false),
    })
}

/// All world sets in a folder (non-recursive), newest first; the server world first.
pub fn scan(dir: &Path) -> Vec<World> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut map: std::collections::BTreeMap<String, World> = Default::default();
    for e in rd.flatten() {
        let Ok(meta) = e.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(c) = FILE_RE.captures(&name) else { continue };
        let id = c[1].to_lowercase();
        let w = map.entry(id.clone()).or_insert_with(|| World {
            is_server_world: id == SERVER_WORLD_ID,
            id: id.clone(),
            slots: vec![],
            latest: None,
            saved_at: None,
            deleted: false,
            size: 0,
            dir: dir.to_path_buf(),
        });
        w.size += meta.len();
        match c.get(2).map(|m| m.as_str()) {
            Some("index") => {
                if let Some(ix) = read_index(&e.path()) {
                    w.latest = Some(ix.latest);
                    w.saved_at = ix.time.or_else(|| meta.modified().ok().map(ms));
                    w.deleted = ix.deleted;
                }
            }
            other => {
                let slot = other.and_then(|s| s.parse().ok()).unwrap_or(0);
                w.slots.push(WorldSlot { file: name, slot, size: meta.len(), modified: meta.modified().map(ms).unwrap_or(0), current: false });
            }
        }
    }
    let mut out: Vec<World> = map
        .into_values()
        .filter(|w| !w.slots.is_empty())
        .map(|mut w| {
            let cur = w.latest.unwrap_or(0);
            for s in w.slots.iter_mut() {
                s.current = s.slot == cur;
            }
            w.slots.sort_by(|a, b| b.modified.cmp(&a.modified));
            if w.saved_at.is_none() {
                w.saved_at = w.slots.first().map(|s| s.modified);
            }
            w
        })
        .collect();
    out.sort_by(|a, b| b.is_server_world.cmp(&a.is_server_world).then(b.saved_at.cmp(&a.saved_at)));
    out
}

pub fn list(inst: &ServerInstance) -> Vec<World> {
    scan(&inst.saves_dir())
}

/// Files of one world set in a folder.
fn world_files(dir: &Path, id: &str) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            FILE_RE.captures(&n).map(|c| c[1].eq_ignore_ascii_case(id)).unwrap_or(false)
        })
        .collect()
}

fn check_id(id: &str) -> AppResult<()> {
    if id.len() == 8 && id.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AppError::msg("Invalid world id"))
    }
}

/// Move one world set into `_quarantine` (recoverable). Returns where it went.
pub fn quarantine_world(dir: &Path, id: &str, label: &str) -> AppResult<Option<PathBuf>> {
    let files = world_files(dir, id);
    if files.is_empty() {
        return Ok(None);
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let q = dir.join("_quarantine").join(format!("{stamp}-{label}-{id}"));
    std::fs::create_dir_all(&q)?;
    for f in files {
        std::fs::rename(&f, q.join(f.file_name().unwrap_or_default()))?;
    }
    Ok(Some(q))
}

/// Copy a world set from another folder (a game client's saves, an old server) in as the
/// server's world, renaming its files to the server container id. The current server world is
/// quarantined first.
pub fn import(inst: &ServerInstance, src_dir: &Path, src_id: &str) -> AppResult<usize> {
    check_id(src_id)?;
    let files = world_files(src_dir, src_id);
    if !files.iter().any(|f| f.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case(src_id)).unwrap_or(false)) && files.len() < 2 {
        return Err(AppError::msg("That folder doesn't contain the world's files."));
    }
    let dest = inst.saves_dir();
    std::fs::create_dir_all(&dest)?;
    quarantine_world(&dest, SERVER_WORLD_ID, "before-import")?;
    let mut n = 0;
    for f in &files {
        let name = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let renamed = format!("{SERVER_WORLD_ID}{}", &name[8..]);
        std::fs::copy(f, dest.join(renamed))?;
        n += 1;
    }
    Ok(n)
}

/// Make an older rolling copy the one the server loads next, by copying it over the current one.
/// The file the index points at is replaced, so the game's own numbering stays untouched.
pub fn restore_slot(inst: &ServerInstance, id: &str, slot: u32) -> AppResult<()> {
    check_id(id)?;
    let dir = inst.saves_dir();
    let world = scan(&dir).into_iter().find(|w| w.id == id).ok_or_else(|| AppError::msg("World not found"))?;
    let src = world.slots.iter().find(|s| s.slot == slot).ok_or_else(|| AppError::msg("That save slot doesn't exist"))?;
    let cur = world.latest.unwrap_or(0);
    if cur == slot {
        return Ok(());
    }
    let cur_name = if cur == 0 { id.to_string() } else { format!("{id}-{cur}") };
    let cur_path = dir.join(&cur_name);
    if cur_path.exists() {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let q = dir.join("_quarantine").join(format!("{stamp}-replaced-slot"));
        std::fs::create_dir_all(&q)?;
        std::fs::copy(&cur_path, q.join(&cur_name))?;
    }
    std::fs::copy(dir.join(&src.file), &cur_path)?;
    Ok(())
}

/// Quarantine every world so the next start generates a brand-new one.
pub fn reset(inst: &ServerInstance) -> AppResult<Option<PathBuf>> {
    quarantine(&inst.saves_dir(), "world-reset")
}

pub fn remove(inst: &ServerInstance, id: &str) -> AppResult<()> {
    check_id(id)?;
    quarantine_world(&inst.saves_dir(), id, "deleted")?;
    Ok(())
}

/// Copy a world set out to a folder (to keep, share or load in another server).
pub fn export(inst: &ServerInstance, id: &str, dest: &Path) -> AppResult<usize> {
    check_id(id)?;
    std::fs::create_dir_all(dest)?;
    let files = world_files(&inst.saves_dir(), id);
    for f in &files {
        std::fs::copy(f, dest.join(f.file_name().unwrap_or_default()))?;
    }
    Ok(files.len())
}

// ---------------------------------------------------------------------------------------------
// Worlds of the Enshrouded game on this PC (to move a singleplayer / hosted world to the server).

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalWorld {
    pub world: World,
    /// Steam Cloud folder or Saved Games
    pub source: String,
}

fn steam_roots() -> Vec<PathBuf> {
    let mut v = vec![PathBuf::from(r"C:\Program Files (x86)\Steam")];
    if let Ok(out) = std::process::Command::new("reg.exe").args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]).output() {
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        if let Some(p) = text.lines().find(|l| l.contains("SteamPath")).and_then(|l| l.split("REG_SZ").nth(1)) {
            let p = PathBuf::from(p.trim().replace('/', "\\"));
            if !v.contains(&p) {
                v.insert(0, p);
            }
        }
    }
    v
}

/// Worlds in the game's save folders: `%USERPROFILE%\Saved Games\Enshrouded` and the Steam Cloud
/// folders `Steam\userdata\<account>\1203620\remote`.
pub fn local_worlds() -> Vec<LocalWorld> {
    let mut out = vec![];
    if let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        for w in scan(&home.join("Saved Games").join("Enshrouded")) {
            out.push(LocalWorld { world: w, source: "Saved Games".into() });
        }
    }
    for root in steam_roots() {
        let Ok(rd) = std::fs::read_dir(root.join("userdata")) else { continue };
        for acct in rd.flatten() {
            let remote = acct.path().join(GAME_APP_ID.to_string()).join("remote");
            for w in scan(&remote) {
                out.push(LocalWorld { world: w, source: format!("Steam Cloud (account {})", acct.file_name().to_string_lossy()) });
            }
        }
    }
    out.sort_by(|a, b| b.world.saved_at.cmp(&a.world.saved_at));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("esm-worlds-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn scans_world_sets_and_index() {
        let d = tmp();
        std::fs::write(d.join("3ad85aea"), b"base").unwrap();
        std::fs::write(d.join("3ad85aea-1"), b"one").unwrap();
        std::fs::write(d.join("3ad85aea-index"), br#"{"latest": 1, "time": 1791976428, "deleted": false}"#).unwrap();
        std::fs::write(d.join("characters"), b"x").unwrap();
        std::fs::write(d.join("5f00aa11"), b"other").unwrap();
        let w = scan(&d);
        assert_eq!(w.len(), 2);
        assert!(w[0].is_server_world);
        assert_eq!(w[0].latest, Some(1));
        assert_eq!(w[0].saved_at, Some(1_791_976_428_000));
        assert!(w[0].slots.iter().find(|s| s.slot == 1).unwrap().current);
        assert_eq!(w[1].id, "5f00aa11");
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn import_renames_and_quarantines() {
        let src = tmp();
        let root = tmp();
        let inst = ServerInstance { install_dir: root.clone(), ..Default::default() };
        std::fs::create_dir_all(inst.saves_dir()).unwrap();
        std::fs::write(inst.saves_dir().join("3ad85aea"), b"old").unwrap();
        std::fs::write(src.join("5f00aa11"), b"mine").unwrap();
        std::fs::write(src.join("5f00aa11-index"), br#"{"latest": 0}"#).unwrap();
        assert_eq!(import(&inst, &src, "5f00aa11").unwrap(), 2);
        assert_eq!(std::fs::read(inst.saves_dir().join("3ad85aea")).unwrap(), b"mine");
        assert!(inst.saves_dir().join("3ad85aea-index").exists());
        assert!(inst.saves_dir().join("_quarantine").exists());
        // Restoring a slot copies it over the current file.
        std::fs::write(inst.saves_dir().join("3ad85aea-2"), b"older").unwrap();
        restore_slot(&inst, "3ad85aea", 2).unwrap();
        assert_eq!(std::fs::read(inst.saves_dir().join("3ad85aea")).unwrap(), b"older");
        let _ = std::fs::remove_dir_all(src);
        let _ = std::fs::remove_dir_all(root);
    }
}
