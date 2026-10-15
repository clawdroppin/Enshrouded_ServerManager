//! Zip backups of a server's world saves (+ enshrouded_server.json), restore, retention and
//! portable server packs (`.enshrouded`).

use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::{now_ms, AppState};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

const MANIFEST: &str = "backup.json";

fn opts() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).compression_level(Some(6))
}

/// Add every file under `dir` to the zip below `prefix/`. Files are read fully first so a save
/// the server is writing at that moment never ends up half-copied mid-stream.
fn add_dir<W: Write + std::io::Seek>(zip: &mut zip::ZipWriter<W>, dir: &Path, prefix: &str, skip: &[&str]) -> AppResult<u32> {
    let mut n = 0;
    if !dir.exists() {
        return Ok(0);
    }
    for e in WalkDir::new(dir).into_iter().flatten() {
        if !e.file_type().is_file() {
            continue;
        }
        let rel = e.path().strip_prefix(dir).unwrap_or(e.path());
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if skip.iter().any(|s| rel_s.starts_with(s)) {
            continue;
        }
        let mut buf = vec![];
        if std::fs::File::open(e.path()).and_then(|mut f| f.read_to_end(&mut buf)).is_err() {
            continue; // locked file – skip rather than fail the whole backup
        }
        zip.start_file(format!("{prefix}/{rel_s}"), opts())?;
        zip.write_all(&buf)?;
        n += 1;
    }
    Ok(n)
}

/// The config file goes in as `Config/enshrouded_server.json`.
fn add_config<W: Write + std::io::Seek>(zip: &mut zip::ZipWriter<W>, inst: &ServerInstance) -> AppResult<u32> {
    let Ok(buf) = std::fs::read(inst.config_path()) else { return Ok(0) };
    zip.start_file(format!("Config/{CONFIG_FILE}"), opts())?;
    zip.write_all(&buf)?;
    Ok(1)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    label: String,
    kind: String,
    created_at: i64,
    server_name: String,
    files: u32,
}

pub async fn create(state: &AppState, inst: &ServerInstance, label: &str, kind: &str) -> AppResult<BackupInfo> {
    let dir = state.backup_dir(inst).await;
    let inst = inst.clone();
    let label = label.to_string();
    let kind = kind.to_string();
    tokio::task::spawn_blocking(move || {
        let info = create_blocking(&inst, &dir, &label, &kind)?;
        prune(&dir, inst.automation.backup_keep);
        Ok(info)
    })
    .await
    .map_err(|e| AppError::msg(e.to_string()))?
}

fn create_blocking(inst: &ServerInstance, dir: &Path, label: &str, kind: &str) -> AppResult<BackupInfo> {
    std::fs::create_dir_all(dir)?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let path = dir.join(format!("{stamp}_{kind}.zip"));
    let tmp = path.with_extension("zip.part");
    let file = std::fs::File::create(&tmp)?;
    let mut zip = zip::ZipWriter::new(file);
    let mut files = add_dir(&mut zip, &inst.saves_dir(), "SaveGames", &["_quarantine"])?;
    files += add_config(&mut zip, inst)?;
    let created_at = now_ms();
    let m = Manifest { label: label.to_string(), kind: kind.to_string(), created_at, server_name: inst.name.clone(), files };
    zip.start_file(MANIFEST, opts())?;
    zip.write_all(&serde_json::to_vec_pretty(&m)?)?;
    zip.finish()?;
    std::fs::rename(&tmp, &path)?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Ok(BackupInfo {
        name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        path,
        size,
        created_at,
        label: label.to_string(),
        kind: kind.to_string(),
        files,
    })
}

fn read_manifest(path: &Path) -> Option<Manifest> {
    let f = std::fs::File::open(path).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let mut e = z.by_name(MANIFEST).ok()?;
    let mut s = String::new();
    e.read_to_string(&mut s).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn list(dir: &Path) -> Vec<BackupInfo> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<BackupInfo> = rd
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("zip")).unwrap_or(false))
        .map(|e| {
            let path = e.path();
            let meta = e.metadata().ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).timestamp_millis())
                .unwrap_or(0);
            let m = read_manifest(&path);
            BackupInfo {
                name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                size,
                created_at: m.as_ref().map(|m| m.created_at).unwrap_or(mtime),
                label: m.as_ref().map(|m| m.label.clone()).unwrap_or_else(|| "Imported".into()),
                kind: m.as_ref().map(|m| m.kind.clone()).unwrap_or_else(|| "manual".into()),
                files: m.as_ref().map(|m| m.files).unwrap_or(0),
                path,
            }
        })
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

/// Keep the newest `keep` automatic backups; manual ones are never pruned.
pub fn prune(dir: &Path, keep: u32) -> u32 {
    if keep == 0 {
        return 0;
    }
    let autos: Vec<BackupInfo> = list(dir).into_iter().filter(|b| b.kind != "manual").collect();
    let mut removed = 0;
    for b in autos.iter().skip(keep as usize) {
        if std::fs::remove_file(&b.path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Move a folder's contents aside into `<parent>/_quarantine/<stamp>-<label>` (never delete).
pub fn quarantine(dir: &Path, label: &str) -> AppResult<Option<PathBuf>> {
    if !dir.exists() || std::fs::read_dir(dir)?.next().is_none() {
        return Ok(None);
    }
    let parent = dir.parent().ok_or_else(|| AppError::msg("bad folder"))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let dest = parent.join("_quarantine").join(format!("{stamp}-{label}"));
    std::fs::create_dir_all(dest.parent().unwrap())?;
    std::fs::rename(dir, &dest)?;
    std::fs::create_dir_all(dir)?;
    Ok(Some(dest))
}

fn extract_prefix(zip_path: &Path, prefix: &str, dest: &Path) -> AppResult<u32> {
    let f = std::fs::File::open(zip_path)?;
    let mut z = zip::ZipArchive::new(f)?;
    let mut n = 0;
    for i in 0..z.len() {
        let mut e = z.by_index(i)?;
        let Some(name) = e.enclosed_name() else { continue };
        let name_s = name.to_string_lossy().replace('\\', "/");
        let Some(rel) = name_s.strip_prefix(&format!("{prefix}/")) else { continue };
        if rel.is_empty() || e.is_dir() {
            continue;
        }
        let out = dest.join(rel);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut w = std::fs::File::create(&out)?;
        std::io::copy(&mut e, &mut w)?;
        n += 1;
    }
    Ok(n)
}

fn zip_has_prefix(zip_path: &Path, prefix: &str) -> bool {
    let Ok(f) = std::fs::File::open(zip_path) else { return false };
    let Ok(z) = zip::ZipArchive::new(f) else { return false };
    let p = format!("{prefix}/");
    let found = z.file_names().any(|n| n.map(|n| n.starts_with(&p)).unwrap_or(false));
    found
}

/// Restore a backup into a stopped server. Current saves are quarantined first; the config (when
/// asked for) goes through the config history like any other edit.
pub fn restore(inst: &ServerInstance, zip_path: &Path, include_config: bool) -> AppResult<u32> {
    let saves = inst.saves_dir();
    quarantine_keep(&saves, "before-restore")?;
    std::fs::create_dir_all(&saves)?;
    let mut n = extract_prefix(zip_path, "SaveGames", &saves)?;
    if include_config {
        n += restore_config(zip_path, inst)?;
    }
    Ok(n)
}

fn restore_config(zip_path: &Path, inst: &ServerInstance) -> AppResult<u32> {
    let f = std::fs::File::open(zip_path)?;
    let mut z = zip::ZipArchive::new(f)?;
    let Ok(mut e) = z.by_name(&format!("Config/{CONFIG_FILE}")) else { return Ok(0) };
    let mut s = String::new();
    e.read_to_string(&mut s)?;
    super::config::write_text(&inst.config_path(), &s)?;
    Ok(1)
}

/// Like `quarantine`, but the save folder's own `_quarantine` stays where it is.
fn quarantine_keep(dir: &Path, label: &str) -> AppResult<()> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Ok(()) };
    let items: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.file_name().map(|n| n != "_quarantine").unwrap_or(false)).collect();
    if items.is_empty() {
        return Ok(());
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let q = dir.join("_quarantine").join(format!("{stamp}-{label}"));
    std::fs::create_dir_all(&q)?;
    for p in items {
        std::fs::rename(&p, q.join(p.file_name().unwrap_or_default()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Server packs (.enshrouded): a zip with manifest.json + Config/ + optional SaveGames/.

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackOptions {
    pub include_saves: bool,
    pub include_secrets: bool,
}

pub fn export_pack(inst: &ServerInstance, dest: &Path, o: &PackOptions) -> AppResult<()> {
    let mut profile = inst.clone();
    if !o.include_secrets {
        for g in profile.config.user_groups.iter_mut() {
            g.password.clear();
        }
        profile.branch_password.clear();
    }
    let tmp = dest.with_extension("part");
    let file = std::fs::File::create(&tmp)?;
    let mut zip = zip::ZipWriter::new(file);
    // Without secrets the pack's config must not carry the passwords either.
    let has_config = if o.include_secrets {
        add_config(&mut zip, inst)? > 0
    } else if let Ok(mut doc) = super::config::read_doc(&inst.config_path()) {
        super::config::apply(&mut doc, &profile.config);
        zip.start_file(format!("Config/{CONFIG_FILE}"), opts())?;
        zip.write_all(super::config::to_text(&doc).as_bytes())?;
        true
    } else {
        false
    };
    let has_saves = if o.include_saves { add_dir(&mut zip, &inst.saves_dir(), "SaveGames", &["_quarantine"])? > 0 } else { false };
    let m = PackManifest {
        format: 1,
        app_version: env!("CARGO_PKG_VERSION").into(),
        exported_at: now_ms(),
        instance: profile,
        has_saves,
        has_config,
        includes_secrets: o.include_secrets,
    };
    zip.start_file("manifest.json", opts())?;
    zip.write_all(&serde_json::to_vec_pretty(&m)?)?;
    zip.finish()?;
    if dest.exists() {
        std::fs::remove_file(dest)?;
    }
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

pub fn read_pack(path: &Path) -> AppResult<PackManifest> {
    let f = std::fs::File::open(path)?;
    let mut z = zip::ZipArchive::new(f).map_err(|_| AppError::msg("That file isn't an Enshrouded server pack."))?;
    let mut e = z.by_name("manifest.json").map_err(|_| AppError::msg("The pack has no manifest.json – not a server pack."))?;
    let mut s = String::new();
    e.read_to_string(&mut s)?;
    Ok(serde_json::from_str(&s)?)
}

/// Unpack the config (and saves) of a pack into a (new) server's install folder.
pub fn unpack_into(path: &Path, inst: &ServerInstance) -> AppResult<()> {
    std::fs::create_dir_all(&inst.install_dir)?;
    restore_config(path, inst)?;
    if zip_has_prefix(path, "SaveGames") {
        let saves = inst.saves_dir();
        std::fs::create_dir_all(&saves)?;
        extract_prefix(path, "SaveGames", &saves)?;
    }
    Ok(())
}

/// Read every entry of a backup and check its CRC – proves the zip can be restored.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub ok: bool,
    pub files: u32,
    pub world_files: u32,
    pub has_config: bool,
    pub bytes: u64,
    pub error: Option<String>,
}

pub fn verify(zip_path: &Path) -> AppResult<VerifyResult> {
    let f = std::fs::File::open(zip_path)?;
    let mut z = zip::ZipArchive::new(f)?;
    let mut out = VerifyResult { ok: true, files: 0, world_files: 0, has_config: false, bytes: 0, error: None };
    for i in 0..z.len() {
        let mut e = z.by_index(i)?;
        if e.is_dir() {
            continue;
        }
        let name = e.name().map(|n| n.to_string()).unwrap_or_default();
        // Reading to the end makes the zip crate check the entry's CRC32.
        match std::io::copy(&mut e, &mut std::io::sink()) {
            Ok(n) => out.bytes += n,
            Err(err) => {
                out.ok = false;
                out.error = Some(format!("{name}: {err}"));
                break;
            }
        }
        out.files += 1;
        if name.starts_with("SaveGames/") {
            let base = name.rsplit('/').next().unwrap_or("");
            if base.len() >= 8 && base[..8].chars().all(|c| c.is_ascii_hexdigit()) {
                out.world_files += 1;
            }
        }
        if name == format!("Config/{CONFIG_FILE}") {
            out.has_config = true;
        }
    }
    if out.ok && out.world_files == 0 {
        out.error = Some("The backup contains no world files.".into());
    }
    Ok(out)
}

pub fn folder_size(dir: &Path) -> u64 {
    WalkDir::new(dir).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_restore_round_trip() {
        let root = std::env::temp_dir().join(format!("esm-bk-{}", uuid::Uuid::new_v4()));
        let inst = ServerInstance { install_dir: root.join("srv"), ..Default::default() };
        std::fs::create_dir_all(inst.saves_dir()).unwrap();
        std::fs::write(inst.saves_dir().join("3ad85aea"), b"world-v1").unwrap();
        std::fs::write(inst.config_path(), br#"{"name": "A"}"#).unwrap();
        let b = create_blocking(&inst, &root.join("bk"), "test", "manual").unwrap();
        assert_eq!(b.files, 2);
        std::fs::write(inst.saves_dir().join("3ad85aea"), b"world-v2").unwrap();
        std::fs::write(inst.config_path(), br#"{"name": "B"}"#).unwrap();
        restore(&inst, &b.path, true).unwrap();
        assert_eq!(std::fs::read(inst.saves_dir().join("3ad85aea")).unwrap(), b"world-v1");
        assert!(std::fs::read_to_string(inst.config_path()).unwrap().contains("\"A\""));
        let listed = list(&root.join("bk"));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].label, "test");
        // The replaced save is quarantined – and not backed up the next time.
        assert!(inst.saves_dir().join("_quarantine").exists());
        let b2 = create_blocking(&inst, &root.join("bk"), "again", "manual").unwrap();
        assert_eq!(b2.files, 2);
        let v = verify(&b2.path).unwrap();
        assert!(v.ok && v.has_config);
        assert_eq!((v.files, v.world_files), (3, 1));
        // A damaged zip is reported, not silently accepted.
        let bytes = std::fs::read(&b2.path).unwrap();
        let bad = root.join("bad.zip");
        std::fs::write(&bad, &bytes[..bytes.len() - 40]).unwrap();
        assert!(verify(&bad).map(|v| !v.ok).unwrap_or(true));
        let _ = std::fs::remove_dir_all(root);
    }
}
