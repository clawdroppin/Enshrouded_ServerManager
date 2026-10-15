//! Every `#[tauri::command]`. Thin wrappers that delegate to `core::*`.

use crate::core::history::{EventRow, MetricRow, PlayerSummary, SessionRow};
use crate::core::{backup, config, diagnostics, network, proc, secrets, server, steamcmd, supervisor, worlds};
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::{manager_log, now_ms, AppState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

type St<'a> = State<'a, Arc<AppState>>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tokio::task::spawn_blocking(f).await.map_err(|e| AppError::msg(e.to_string()))?
}

/// Run a long job in the background; errors become toasts. Returns immediately.
fn background<F>(app: &AppHandle, id: &str, what: &'static str, fut: F)
where
    F: std::future::Future<Output = AppResult<()>> + Send + 'static,
{
    let (app2, id) = (app.clone(), id.to_string());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = fut.await {
            crate::state::notify(&app2, &id, "error", format!("{what} failed: {e}"));
        }
    });
}

fn require_stopped(inst: &ServerInstance) -> AppResult<()> {
    let (_, main) = server::running_pids(inst);
    if main.is_some() {
        return Err(AppError::msg("Stop the server first – it has these files open while running."));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------- app

#[tauri::command]
pub async fn get_app_info(state: St<'_>) -> AppResult<AppInfo> {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: state.data_dir.clone(),
        portable: state.portable,
        app_id: SERVER_APP_ID,
        cores: sys.cpus().len(),
        total_memory: sys.total_memory(),
        os: sysinfo::System::long_os_version().unwrap_or_else(|| "Windows".into()),
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        lost_secrets: state.lost_secrets.clone(),
    })
}

#[tauri::command]
pub async fn get_settings(state: St<'_>) -> AppResult<AppSettings> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn save_settings(state: St<'_>, settings: AppSettings) -> AppResult<()> {
    let mut s = settings;
    s.last_data_dir = Some(state.data_dir.clone());
    s.metrics_retention_days = s.metrics_retention_days.clamp(1, 365);
    *state.settings.write().await = s;
    state.persist_settings().await
}

#[tauri::command]
pub async fn test_webhook(state: St<'_>, url: String) -> AppResult<()> {
    crate::state::send_webhook(&state.http, url.trim(), "success", "", "Test message from Enshrouded Server Manager \u{2013} notifications work!").await
}

#[tauri::command]
pub fn app_quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn window_hide(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
pub async fn cancel_task(state: St<'_>, task_id: String) -> AppResult<()> {
    if let Some(f) = state.cancels.lock().await.get(&task_id) {
        f.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderProbe {
    exists: bool,
    writable: bool,
    empty: bool,
    has_server: bool,
    free_bytes: Option<u64>,
}

#[tauri::command]
pub async fn probe_folder(path: String) -> AppResult<FolderProbe> {
    blocking(move || {
        let p = PathBuf::from(&path);
        let exists = p.exists();
        let empty = !exists || std::fs::read_dir(&p).map(|mut r| r.next().is_none()).unwrap_or(true);
        let has_server = resolve_server_exe(&p).exists();
        let probe_dir = if exists { p.clone() } else { p.ancestors().find(|a| a.exists()).map(|a| a.to_path_buf()).unwrap_or(p.clone()) };
        let t = probe_dir.join(format!(".esm-{}", uuid::Uuid::new_v4()));
        let writable = std::fs::write(&t, b"").is_ok();
        let _ = std::fs::remove_file(&t);
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let n = proc::norm(&probe_dir);
        let free_bytes = disks
            .iter()
            .filter(|d| n.starts_with(&proc::norm(d.mount_point())))
            .max_by_key(|d| d.mount_point().as_os_str().len())
            .map(|d| d.available_space());
        Ok(FolderProbe { exists, writable, empty, has_server, free_bytes })
    })
    .await
}

// ---------------------------------------------------------------------------------- instances

#[tauri::command]
pub async fn list_instances(state: St<'_>) -> AppResult<Vec<ServerInstance>> {
    Ok(state.instances.read().await.clone())
}

#[tauri::command]
pub async fn get_statuses(state: St<'_>) -> AppResult<Vec<ServerStatus>> {
    let list = state.instances.read().await.clone();
    let mut out = vec![];
    for i in &list {
        out.push(state.status(i).await);
    }
    Ok(out)
}

#[tauri::command]
pub async fn suggest_port(state: St<'_>, exclude_id: Option<String>) -> AppResult<u16> {
    let list = state.instances.read().await.clone();
    Ok(network::suggest_port(&list, exclude_id.as_deref()).await)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInput {
    name: String,
    install_dir: Option<PathBuf>,
    color: Option<String>,
    config: ServerConfig,
    install_now: bool,
    template_id: Option<String>,
}

/// Folders must be absolute – a relative path would resolve against wherever the app (or
/// SteamCMD, which then silently falls back to its own library) happens to run.
fn abs_dir(p: PathBuf) -> AppResult<PathBuf> {
    let s = p.to_string_lossy().trim().replace('/', "\\").trim_end_matches('\\').to_string();
    let s = if s.len() == 2 && s.ends_with(':') { format!("{s}\\") } else { s };
    let p = PathBuf::from(&s);
    if !p.is_absolute() {
        return Err(AppError::msg(format!("“{s}” isn't a full folder path – use something like D:\\EnshroudedServers\\MyServer.")));
    }
    Ok(p)
}

fn slug(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() {
        "server".into()
    } else {
        s.chars().take(40).collect()
    }
}

async fn unique_dir(state: &AppState, name: &str) -> PathBuf {
    let root = state.default_install_root().await;
    let base = slug(name);
    let mut p = root.join(&base);
    let mut n = 2;
    let used: Vec<String> = state.instances.read().await.iter().map(|i| proc::norm(&i.install_dir)).collect();
    while p.exists() && std::fs::read_dir(&p).map(|mut r| r.next().is_some()).unwrap_or(false) || used.contains(&proc::norm(&p)) {
        p = root.join(format!("{base}-{n}"));
        n += 1;
    }
    p
}

async fn add_instance(state: &AppState, inst: ServerInstance) -> AppResult<ServerInstance> {
    {
        let mut list = state.instances.write().await;
        if list.iter().any(|i| proc::norm(&i.install_dir) == proc::norm(&inst.install_dir)) {
            return Err(AppError::msg("Another server profile already uses that folder."));
        }
        list.push(inst.clone());
    }
    state.persist_instances().await?;
    Ok(inst)
}

/// Roles without a password get a strong random one (a fresh server does the same).
fn fill_passwords(c: &mut ServerConfig) {
    for g in c.user_groups.iter_mut() {
        if g.password.is_empty() {
            g.password = secrets::random_password(14);
        }
    }
}

#[tauri::command]
pub async fn create_instance(app: AppHandle, state: St<'_>, input: CreateInput) -> AppResult<ServerInstance> {
    let name = if input.name.trim().is_empty() { "Enshrouded Server".to_string() } else { input.name.trim().to_string() };
    let mut inst = match &input.template_id {
        Some(t) => state.instance(t).await?,
        None => ServerInstance::default(),
    };
    inst.id = uuid::Uuid::new_v4().to_string();
    inst.name = name.clone();
    inst.created_at = now_ms();
    inst.color = input.color.unwrap_or_else(|| "flame".into());
    inst.install_dir = match input.install_dir {
        Some(d) if !d.as_os_str().is_empty() => abs_dir(d)?,
        _ => unique_dir(&state, &name).await,
    };
    inst.config = input.config;
    let list = state.instances.read().await.clone();
    if inst.config.query_port == 0 || list.iter().any(|i| i.config.query_port == inst.config.query_port) {
        inst.config.query_port = network::suggest_port(&list, None).await;
    }
    std::fs::create_dir_all(&inst.install_dir)?;
    // An install that already exists in the chosen folder keeps its world, roles and bans.
    if inst.config_path().exists() {
        let mut probe = inst.config.clone();
        if config::read_file_into(&inst.config_path(), &mut probe).is_ok() && !probe.user_groups.is_empty() {
            inst.config.user_groups = probe.user_groups;
        }
    }
    fill_passwords(&mut inst.config);
    config::ensure_valid(&inst.config)?;
    let _ = config::sync(&inst);
    let inst = add_instance(&state, inst).await?;
    if input.install_now {
        let (st, app2, id) = (state.inner().clone(), app.clone(), inst.id.clone());
        background(&app, &inst.id, "Install", async move { server::update_job(&st, &app2, &id, false).await });
    }
    state.emit_status(&app, &inst.id).await;
    Ok(inst)
}

#[tauri::command]
pub async fn update_instance(app: AppHandle, state: St<'_>, instance: ServerInstance) -> AppResult<ServerInstance> {
    let old = state.instance(&instance.id).await?;
    let mut inst = instance;
    inst.install_dir = abs_dir(inst.install_dir)?;
    inst.config.slot_count = inst.config.slot_count.clamp(1, MAX_SLOTS);
    if inst.config.query_port == 0 {
        return Err(AppError::msg("The query port can't be 0."));
    }
    config::ensure_valid(&inst.config)?;
    if proc::norm(&old.install_dir) != proc::norm(&inst.install_dir) {
        require_stopped(&old).map_err(|_| AppError::msg("Stop the server before changing its folder."))?;
    }
    state.put_instance(&inst).await?;
    let build = steamcmd::local_build(&inst.install_dir);
    state.with_runtime(&inst.id, |r| r.installed_build = build).await;
    let (_, main) = server::running_pids(&inst);
    let restart_needed = old.config != inst.config || old.launch.extra_args != inst.launch.extra_args;
    match main {
        Some(pid) => {
            // Priority / affinity / console visibility apply live; the config at the next start.
            if old.launch.priority != inst.launch.priority || old.launch.affinity != inst.launch.affinity {
                let _ = proc::apply_priority_affinity(pid, &inst.launch.priority, &inst.launch.affinity);
            }
            if old.launch.show_console != inst.launch.show_console {
                let (pids, _) = server::running_pids(&inst);
                proc::set_console_visible(&pids, inst.launch.show_console);
                state.with_runtime(&inst.id, |r| r.console_hidden = !inst.launch.show_console).await;
            }
            if restart_needed {
                state.with_runtime(&inst.id, |r| r.pending_restart = true).await;
            }
        }
        None => {
            if inst.is_installed() || inst.config_path().exists() {
                let i2 = inst.clone();
                blocking(move || config::sync(&i2)).await?;
            }
        }
    }
    if old.config != inst.config {
        state.history.event(Some(&inst.id), "info", "config", "Server settings changed");
    }
    state.emit_status(&app, &inst.id).await;
    let _ = app.emit("instances-changed", ());
    Ok(inst)
}

#[tauri::command]
pub async fn validate_config(config: ServerConfig) -> AppResult<Vec<config::Issue>> {
    Ok(config::validate(&config))
}

#[tauri::command]
pub async fn reorder_instances(state: St<'_>, ids: Vec<String>) -> AppResult<()> {
    {
        let mut list = state.instances.write().await;
        list.sort_by_key(|i| ids.iter().position(|x| x == &i.id).unwrap_or(usize::MAX));
    }
    state.persist_instances().await
}

#[tauri::command]
pub async fn delete_instance(app: AppHandle, state: St<'_>, id: String, delete_files: bool) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    let (_, main) = server::running_pids(&inst);
    if main.is_some() {
        server::stop(&state, &app, &id, false).await?;
    }
    state.instances.write().await.retain(|i| i.id != id);
    state.persist_instances().await?;
    state.runtime.lock().await.remove(&id);
    state.history.forget(&id);
    if delete_files && inst.install_dir.exists() {
        // Keep backups: they live inside the folder by default, so move them out first.
        let dir = inst.install_dir.clone();
        let keep = state.data_dir.join("deleted-server-backups").join(&id);
        blocking(move || {
            let bk = dir.join("Backups");
            if bk.exists() {
                std::fs::create_dir_all(keep.parent().unwrap())?;
                let _ = std::fs::rename(&bk, &keep);
            }
            std::fs::remove_dir_all(&dir)?;
            Ok(())
        })
        .await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn clone_instance(state: St<'_>, id: String, name: String, copy_saves: bool) -> AppResult<ServerInstance> {
    let src = state.instance(&id).await?;
    let mut inst = src.clone();
    inst.id = uuid::Uuid::new_v4().to_string();
    inst.name = name.trim().to_string();
    inst.config.name = inst.name.clone();
    inst.created_at = now_ms();
    inst.install_dir = unique_dir(&state, &inst.name).await;
    let list = state.instances.read().await.clone();
    inst.config.query_port = network::suggest_port(&list, None).await;
    // New passwords: a copy shouldn't let the original's players in.
    for g in inst.config.user_groups.iter_mut() {
        g.password = secrets::random_password(14);
    }
    let (s2, d2) = (src.clone(), inst.clone());
    blocking(move || {
        std::fs::create_dir_all(&d2.install_dir)?;
        // Start from the original's file (unknown keys), but without its ban list.
        if let Ok(mut doc) = config::read_doc(&s2.config_path()) {
            doc.insert("bannedAccounts".into(), serde_json::Value::Array(vec![]));
            config::apply(&mut doc, &d2.config);
            config::write_text(&d2.config_path(), &config::to_text(&doc))?;
        }
        if copy_saves && s2.saves_dir().exists() {
            for w in worlds::list(&s2) {
                worlds::export(&s2, &w.id, &d2.saves_dir())?;
            }
        }
        config::sync(&d2)
    })
    .await?;
    add_instance(&state, inst).await
}

#[tauri::command]
pub async fn find_existing_installs(state: St<'_>) -> AppResult<Vec<PathBuf>> {
    let known: Vec<String> = state.instances.read().await.iter().map(|i| proc::norm(&i.install_dir)).collect();
    Ok(steamcmd::find_steam_client_installs().await.into_iter().filter(|p| !known.contains(&proc::norm(p))).collect())
}

#[tauri::command]
pub async fn import_existing(app: AppHandle, state: St<'_>, path: PathBuf, name: Option<String>) -> AppResult<ServerInstance> {
    let path = abs_dir(path)?;
    if !resolve_server_exe(&path).exists() {
        return Err(AppError::msg("No Enshrouded dedicated server found in that folder (looked for enshrouded_server.exe)."));
    }
    let mut inst = ServerInstance { id: uuid::Uuid::new_v4().to_string(), created_at: now_ms(), install_dir: path.clone(), ..Default::default() };
    config::read_file_into(&inst.config_path(), &mut inst.config)?;
    inst.name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| inst.config.name.clone());
    let list = state.instances.read().await.clone();
    if list.iter().any(|i| i.config.query_port == inst.config.query_port) {
        inst.config.query_port = network::suggest_port(&list, None).await;
    }
    let inst = add_instance(&state, inst).await?;
    let build = steamcmd::local_build(&inst.install_dir);
    state.with_runtime(&inst.id, |r| r.installed_build = build).await;
    state.emit_status(&app, &inst.id).await;
    Ok(inst)
}

// ---------------------------------------------------------------------------------- lifecycle

#[tauri::command]
pub async fn install_server(app: AppHandle, state: St<'_>, id: String, validate: bool) -> AppResult<()> {
    let st = state.inner().clone();
    let (app2, id2) = (app.clone(), id.clone());
    background(&app, &id, if validate { "Validate" } else { "Update" }, async move { server::update_job(&st, &app2, &id2, validate).await });
    Ok(())
}

#[tauri::command]
pub async fn check_update(app: AppHandle, state: St<'_>, id: String) -> AppResult<bool> {
    let st = state.inner().clone();
    state.latest_build_cache.lock().await.clear();
    supervisor::check_update(&st, &app, &id, false).await
}

#[tauri::command]
pub async fn start_server(app: AppHandle, state: St<'_>, id: String) -> AppResult<()> {
    state.with_runtime(&id, |r| r.crash_loop = false).await;
    server::start(&state, &app, &id).await
}

#[tauri::command]
pub async fn stop_server(app: AppHandle, state: St<'_>, id: String, force: bool) -> AppResult<()> {
    let st = state.inner().clone();
    let (app2, id2) = (app.clone(), id.clone());
    // Graceful stops take a few seconds (world save) – return at once, status events follow.
    background(&app, &id, "Stop", async move { server::stop(&st, &app2, &id2, force).await });
    Ok(())
}

#[tauri::command]
pub async fn restart_server(app: AppHandle, state: St<'_>, id: String, update: bool) -> AppResult<()> {
    let st = state.inner().clone();
    let (app2, id2) = (app.clone(), id.clone());
    background(&app, &id, "Restart", async move { server::restart(&st, &app2, &id2, update).await });
    Ok(())
}

#[tauri::command]
pub async fn cancel_deferred(app: AppHandle, state: St<'_>, id: String) -> AppResult<()> {
    state.with_runtime(&id, |r| r.waiting_empty = None).await;
    manager_log(&app, &id, "Cancelled the action that was waiting for players to leave.");
    state.emit_status(&app, &id).await;
    Ok(())
}

#[tauri::command]
pub async fn get_launch_line(state: St<'_>, id: String) -> AppResult<String> {
    Ok(server::launch_line(&state.instance(&id).await?))
}

#[tauri::command]
pub async fn set_console_visible(state: St<'_>, id: String, visible: bool) -> AppResult<usize> {
    let inst = state.instance(&id).await?;
    let (pids, _) = server::running_pids(&inst);
    state.with_runtime(&id, |r| r.console_hidden = !visible).await;
    Ok(proc::set_console_visible(&pids, visible))
}

#[tauri::command]
pub async fn clear_crash_state(app: AppHandle, state: St<'_>, id: String) -> AppResult<()> {
    state
        .with_runtime(&id, |r| {
            r.crash_loop = false;
            r.crash_times.clear();
            r.restart_at = None;
            if r.state == Some(ServerState::Crashed) {
                r.state = Some(ServerState::Stopped);
            }
        })
        .await;
    state.emit_status(&app, &id).await;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    info: Option<crate::core::a2s::A2sInfo>,
    players: Vec<crate::core::a2s::A2sPlayer>,
}

/// Ask the server what it tells the Steam browser (A2S) – on this PC, or at an address.
#[tauri::command]
pub async fn query_server(state: St<'_>, id: String, address: Option<String>) -> AppResult<QueryResult> {
    use crate::core::a2s;
    let inst = state.instance(&id).await?;
    let addr: std::net::SocketAddr = match address.filter(|a| !a.trim().is_empty()) {
        Some(a) => tokio::net::lookup_host(a.trim()).await?.next().ok_or_else(|| AppError::msg("Address not found"))?,
        None => a2s::local_addr(&inst.config.ip, inst.config.query_port),
    };
    let t = std::time::Duration::from_secs(3);
    let info = a2s::info(addr, t).await;
    let players = if info.is_some() { a2s::players(addr, t).await.unwrap_or_default() } else { vec![] };
    Ok(QueryResult { info, players })
}

// ------------------------------------------------------------------------------------- config

#[tauri::command]
pub async fn read_config(state: St<'_>, id: String) -> AppResult<ConfigFile> {
    let inst = state.instance(&id).await?;
    let pending = state.with_runtime(&id, |r| r.pending_raw.clone()).await;
    let path = inst.config_path();
    blocking(move || {
        let meta = std::fs::metadata(&path).ok();
        let is_pending = pending.is_some();
        Ok(ConfigFile {
            exists: meta.is_some(),
            modified: meta.and_then(|m| m.modified().ok()).map(|t| chrono::DateTime::<chrono::Utc>::from(t).timestamp_millis()),
            text: match pending {
                Some(t) => t,
                None => std::fs::read_to_string(&path).unwrap_or_default(),
            },
            pending: is_pending,
            path,
        })
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResult {
    deferred: bool,
    instance: ServerInstance,
}

/// Save hand edits of enshrouded_server.json. The profile follows the file's managed keys.
#[tauri::command]
pub async fn write_config(app: AppHandle, state: St<'_>, id: String, text: String) -> AppResult<WriteResult> {
    let mut inst = state.instance(&id).await?;
    let doc: serde_json::Value = serde_json::from_str(&text).map_err(|e| AppError::msg(format!("JSON error at line {}, column {}: {e}", e.line(), e.column())))?;
    let Some(obj) = doc.as_object() else { return Err(AppError::msg("The file must be a JSON object.")) };
    let mut probe = inst.config.clone();
    config::read_into(obj, &mut probe);
    config::ensure_valid(&probe)?;
    let (_, main) = server::running_pids(&inst);
    let deferred = main.is_some();
    if deferred {
        state
            .with_runtime(&id, |r| {
                r.pending_raw = Some(text.clone());
                r.pending_restart = true;
            })
            .await;
    } else {
        let p = inst.config_path();
        let t = text.clone();
        blocking(move || config::write_text(&p, &t)).await?;
    }
    inst.config = probe;
    state.put_instance(&inst).await?;
    let _ = app.emit("instances-changed", ());
    state.emit_status(&app, &id).await;
    Ok(WriteResult { deferred, instance: inst })
}

#[tauri::command]
pub async fn discard_pending_config(app: AppHandle, state: St<'_>, id: String) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    state.with_runtime(&id, |r| r.pending_raw = None).await;
    // The profile goes back to what the file says.
    let mut back = inst.clone();
    config::read_file_into(&inst.config_path(), &mut back.config)?;
    state.put_instance(&back).await?;
    let _ = app.emit("instances-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn list_config_history(state: St<'_>, id: String) -> AppResult<Vec<(String, i64)>> {
    let inst = state.instance(&id).await?;
    let dir = config::history_dir(&inst.config_path());
    blocking(move || {
        let mut v: Vec<(String, i64)> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| {
                        let t = e.metadata().ok().and_then(|m| m.modified().ok()).map(|t| chrono::DateTime::<chrono::Utc>::from(t).timestamp_millis()).unwrap_or(0);
                        (e.path().to_string_lossy().into_owned(), t)
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(v)
    })
    .await
}

#[tauri::command]
pub async fn read_text_file(state: St<'_>, id: String, path: PathBuf) -> AppResult<String> {
    let inst = state.instance(&id).await?;
    if !proc::is_within(&path, &inst.install_dir) && !proc::is_within(&path, &inst.logs_dir()) {
        return Err(AppError::msg("That file is outside the server folder."));
    }
    blocking(move || Ok(String::from_utf8_lossy(&std::fs::read(&path)?).into_owned())).await
}

#[tauri::command]
pub async fn list_bans(state: St<'_>, id: String) -> AppResult<Vec<config::BanEntry>> {
    let inst = state.instance(&id).await?;
    blocking(move || config::read_bans(&inst.config_path())).await
}

#[tauri::command]
pub async fn remove_ban(state: St<'_>, id: String, raw: String) -> AppResult<bool> {
    let inst = state.instance(&id).await?;
    require_stopped(&inst).map_err(|_| AppError::msg("Stop the server first – it keeps the ban list in memory and writes it back on changes. (Or unban in-game from the Social tab.)"))?;
    blocking(move || config::remove_ban(&inst.config_path(), &raw)).await
}

// ---------------------------------------------------------------------------------------- logs

#[tauri::command]
pub async fn read_log_tail(state: St<'_>, id: String, max_lines: usize) -> AppResult<Vec<String>> {
    let inst = state.instance(&id).await?;
    blocking(move || {
        let path = inst.log_path();
        let Ok(meta) = std::fs::metadata(&path) else { return Ok(vec![]) };
        use std::io::{Read, Seek, SeekFrom};
        let len = meta.len();
        let start = len.saturating_sub(1_048_576);
        let mut f = std::fs::File::open(&path)?;
        f.seek(SeekFrom::Start(start))?;
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        let skip = lines.len().saturating_sub(max_lines.clamp(10, 20_000));
        Ok(lines[skip..].to_vec())
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFile {
    path: PathBuf,
    name: String,
    size: u64,
    modified: i64,
}

/// The current log plus the previous sessions the server keeps in `logs\backup`.
#[tauri::command]
pub async fn list_log_files(state: St<'_>, id: String) -> AppResult<Vec<LogFile>> {
    let inst = state.instance(&id).await?;
    blocking(move || {
        let mut v: Vec<LogFile> = vec![];
        for dir in [inst.logs_dir(), inst.logs_dir().join("backup")] {
            let Ok(rd) = std::fs::read_dir(dir) else { continue };
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if !n.starts_with("enshrouded_server") || !n.ends_with(".log") {
                    continue;
                }
                let Ok(m) = e.metadata() else { continue };
                v.push(LogFile {
                    name: n,
                    path: e.path(),
                    size: m.len(),
                    modified: m.modified().ok().map(|t| chrono::DateTime::<chrono::Utc>::from(t).timestamp_millis()).unwrap_or(0),
                });
            }
        }
        v.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(v)
    })
    .await
}

// -------------------------------------------------------------------------------------- worlds

#[tauri::command]
pub async fn list_worlds(state: St<'_>, id: String) -> AppResult<Vec<worlds::World>> {
    let inst = state.instance(&id).await?;
    blocking(move || Ok(worlds::list(&inst))).await
}

#[tauri::command]
pub async fn list_local_worlds() -> AppResult<Vec<worlds::LocalWorld>> {
    blocking(|| Ok(worlds::local_worlds())).await
}

/// World sets in any folder the user picks (an old server's savegame folder, a copied save).
#[tauri::command]
pub async fn scan_world_folder(path: PathBuf) -> AppResult<Vec<worlds::World>> {
    blocking(move || Ok(worlds::scan(&path))).await
}

#[tauri::command]
pub async fn import_world(app: AppHandle, state: St<'_>, id: String, dir: PathBuf, world_id: String) -> AppResult<usize> {
    let inst = state.instance(&id).await?;
    require_stopped(&inst)?;
    if !worlds::list(&inst).is_empty() {
        let _ = backup::create(&state, &inst, "Before world import", "auto").await;
    }
    let n = blocking(move || worlds::import(&inst, &dir, &world_id)).await?;
    crate::state::notify_kind(&app, &id, "success", "world", format!("Imported a world ({n} files) – it loads on the next start"));
    Ok(n)
}

#[tauri::command]
pub async fn restore_world_slot(state: St<'_>, id: String, world_id: String, slot: u32) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    require_stopped(&inst)?;
    let _ = backup::create(&state, &inst, "Before autosave restore", "auto").await;
    blocking(move || worlds::restore_slot(&inst, &world_id, slot)).await
}

#[tauri::command]
pub async fn export_world(state: St<'_>, id: String, world_id: String, dest: PathBuf) -> AppResult<usize> {
    let inst = state.instance(&id).await?;
    blocking(move || worlds::export(&inst, &world_id, &dest)).await
}

#[tauri::command]
pub async fn delete_world(state: St<'_>, id: String, world_id: String) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    require_stopped(&inst)?;
    blocking(move || worlds::remove(&inst, &world_id)).await
}

#[tauri::command]
pub async fn reset_world(state: St<'_>, id: String) -> AppResult<Option<PathBuf>> {
    let inst = state.instance(&id).await?;
    require_stopped(&inst)?;
    let _ = backup::create(&state, &inst, "Before world reset", "auto").await;
    blocking(move || worlds::reset(&inst)).await
}

// ------------------------------------------------------------------------------------- history

#[tauri::command]
pub async fn metrics_history(state: St<'_>, id: String, since: i64) -> AppResult<Vec<MetricRow>> {
    Ok(state.history.metrics(&id, since))
}

#[tauri::command]
pub async fn event_history(state: St<'_>, id: Option<String>, limit: u32) -> AppResult<Vec<EventRow>> {
    Ok(state.history.events(id.as_deref(), limit.clamp(1, 2000)))
}

#[tauri::command]
pub async fn player_sessions(state: St<'_>, id: String, limit: u32) -> AppResult<Vec<SessionRow>> {
    Ok(state.history.sessions(&id, limit.clamp(1, 2000)))
}

#[tauri::command]
pub async fn known_players(state: St<'_>, id: String) -> AppResult<Vec<PlayerSummary>> {
    Ok(state.history.players(&id))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRow {
    build_id: String,
    branch: String,
    ts: i64,
}

#[tauri::command]
pub async fn build_history(state: St<'_>, id: String) -> AppResult<Vec<BuildRow>> {
    Ok(state.history.builds(&id).into_iter().map(|(build_id, branch, ts)| BuildRow { build_id, branch, ts }).collect())
}

// ------------------------------------------------------------------------------------- backups

#[tauri::command]
pub async fn create_backup(app: AppHandle, state: St<'_>, id: String, label: String) -> AppResult<BackupInfo> {
    let inst = state.instance(&id).await?;
    state.set_busy(&app, &id, Some("Backing up")).await;
    let res = backup::create(&state, &inst, if label.trim().is_empty() { "Manual backup" } else { label.trim() }, "manual").await;
    state.set_busy(&app, &id, None).await;
    let b = res?;
    state.with_runtime(&id, |r| r.last_backup = b.created_at).await;
    let _ = app.emit("backup-created", &b);
    Ok(b)
}

#[tauri::command]
pub async fn list_backups(state: St<'_>, id: String) -> AppResult<Vec<BackupInfo>> {
    let inst = state.instance(&id).await?;
    let dir = state.backup_dir(&inst).await;
    blocking(move || Ok(backup::list(&dir))).await
}

#[tauri::command]
pub async fn get_backup_dir(state: St<'_>, id: String) -> AppResult<PathBuf> {
    let inst = state.instance(&id).await?;
    Ok(state.backup_dir(&inst).await)
}

#[tauri::command]
pub async fn restore_backup(app: AppHandle, state: St<'_>, id: String, path: PathBuf, include_config: bool) -> AppResult<u32> {
    let inst = state.instance(&id).await?;
    require_stopped(&inst)?;
    let dir = state.backup_dir(&inst).await;
    if !proc::is_within(&path, &dir) {
        return Err(AppError::msg("That backup isn't in this server's backup folder."));
    }
    // Safety net: snapshot what's there now.
    if inst.saves_dir().exists() {
        let _ = backup::create(&state, &inst, "Before restore", "auto").await;
    }
    let i2 = inst.clone();
    let n = blocking(move || backup::restore(&i2, &path, include_config)).await?;
    if include_config {
        // The profile follows the restored config file.
        let mut updated = inst.clone();
        if config::read_file_into(&inst.config_path(), &mut updated.config).is_ok() {
            state.put_instance(&updated).await?;
            let _ = app.emit("instances-changed", ());
        }
    }
    manager_log(&app, &id, format!("Restored {n} files from backup."));
    crate::state::notify_kind(&app, &id, "success", "backup", format!("Restored {n} files from a backup"));
    Ok(n)
}

#[tauri::command]
pub async fn verify_backup(state: St<'_>, id: String, path: PathBuf) -> AppResult<backup::VerifyResult> {
    let inst = state.instance(&id).await?;
    let dir = state.backup_dir(&inst).await;
    if !proc::is_within(&path, &dir) {
        return Err(AppError::msg("That backup isn't in this server's backup folder."));
    }
    blocking(move || backup::verify(&path)).await
}

#[tauri::command]
pub async fn delete_backup(state: St<'_>, id: String, path: PathBuf) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    let dir = state.backup_dir(&inst).await;
    if !proc::is_within(&path, &dir) {
        return Err(AppError::msg("That backup isn't in this server's backup folder."));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

// --------------------------------------------------------------------------------------- packs

#[tauri::command]
pub async fn export_pack(state: St<'_>, id: String, dest: PathBuf, options: backup::PackOptions) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    blocking(move || backup::export_pack(&inst, &dest, &options)).await
}

#[tauri::command]
pub async fn read_pack(path: PathBuf) -> AppResult<PackManifest> {
    blocking(move || backup::read_pack(&path)).await
}

#[tauri::command]
pub async fn import_pack(app: AppHandle, state: St<'_>, path: PathBuf, name: String, install_dir: Option<PathBuf>, install_now: bool) -> AppResult<ServerInstance> {
    let p2 = path.clone();
    let m = blocking(move || backup::read_pack(&p2)).await?;
    let mut inst = m.instance;
    inst.id = uuid::Uuid::new_v4().to_string();
    inst.created_at = now_ms();
    inst.name = if name.trim().is_empty() { inst.name } else { name.trim().to_string() };
    inst.install_dir = match install_dir {
        Some(d) if !d.as_os_str().is_empty() => abs_dir(d)?,
        _ => unique_dir(&state, &inst.name).await,
    };
    let list = state.instances.read().await.clone();
    if list.iter().any(|i| i.config.query_port == inst.config.query_port) {
        inst.config.query_port = network::suggest_port(&list, None).await;
    }
    fill_passwords(&mut inst.config);
    let i2 = inst.clone();
    blocking(move || {
        backup::unpack_into(&path, &i2)?;
        config::sync(&i2)
    })
    .await?;
    let inst = add_instance(&state, inst).await?;
    if install_now && !inst.is_installed() {
        let (st, app2, id) = (state.inner().clone(), app.clone(), inst.id.clone());
        background(&app, &inst.id, "Install", async move { server::update_job(&st, &app2, &id, false).await });
    }
    state.emit_status(&app, &inst.id).await;
    Ok(inst)
}

// ------------------------------------------------------------------------------------- network

#[tauri::command]
pub async fn firewall_state(state: St<'_>, id: String) -> AppResult<network::FirewallState> {
    let inst = state.instance(&id).await?;
    Ok(network::firewall_state(&inst).await)
}

#[tauri::command]
pub async fn add_firewall_rules(state: St<'_>, id: String) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    let out = network::add_firewall_rules(&inst, &state.data_dir.join("tmp")).await?;
    if !out.contains("RULES_OK") {
        return Err(AppError::msg(format!("Windows didn't confirm the rules: {}", out.lines().filter(|l| l.contains("ERROR") || l.contains("Error")).collect::<Vec<_>>().join(" "))));
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_firewall_rules(state: St<'_>, id: String) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    network::remove_firewall_rules(&inst, &state.data_dir.join("tmp")).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetInfo {
    public_ip: Option<String>,
    local: Vec<network::LocalIp>,
    gateway: Option<String>,
}

#[tauri::command]
pub async fn net_info(state: St<'_>) -> AppResult<NetInfo> {
    let (public_ip, gateway) = tokio::join!(network::public_ip(&state.http), network::default_gateway());
    let mut local = blocking(|| Ok(network::local_ips())).await?;
    // Put the adapter on the router's subnet first – that's the address to forward to.
    if let Some(gw) = &gateway {
        let prefix: String = gw.rsplitn(2, '.').nth(1).unwrap_or("").to_string() + ".";
        local.sort_by_key(|l| !l.ip.starts_with(&prefix));
    }
    Ok(NetInfo { public_ip, local, gateway })
}

/// Servers Steam lists at this PC's public IP – what the in-game Join list is built from.
#[tauri::command]
pub async fn steam_listing(state: St<'_>, id: String) -> AppResult<Vec<network::SteamListing>> {
    let ip = match state.with_runtime(&id, |r| r.public_ip.clone()).await {
        Some(ip) => ip,
        None => network::public_ip(&state.http).await.ok_or_else(|| AppError::msg("Couldn't find this PC's public IP."))?,
    };
    network::steam_listing(&state.http, &ip).await
}

#[tauri::command]
pub async fn upnp_probe(state: St<'_>) -> AppResult<crate::core::upnp::UpnpInfo> {
    Ok(crate::core::upnp::probe(&state.http).await)
}

#[tauri::command]
pub async fn upnp_map(app: AppHandle, state: St<'_>, id: String) -> AppResult<Vec<String>> {
    let inst = state.instance(&id).await?;
    let lines = crate::core::upnp::map_server(&inst).await?;
    manager_log(&app, &id, format!("UPnP: {}", lines.join(", ")));
    Ok(lines)
}

#[tauri::command]
pub async fn upnp_unmap(state: St<'_>, id: String) -> AppResult<()> {
    let inst = state.instance(&id).await?;
    crate::core::upnp::unmap_server(&inst).await
}

// --------------------------------------------------------------------------------- diagnostics

#[tauri::command]
pub async fn run_diagnostics(state: St<'_>, id: String) -> AppResult<Vec<DiagCheck>> {
    let inst = state.instance(&id).await?;
    Ok(diagnostics::run(&state, &inst).await)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    install: u64,
    saves: u64,
    backups: u64,
    logs: u64,
}

#[tauri::command]
pub async fn disk_usage(state: St<'_>, id: String) -> AppResult<DiskUsage> {
    let inst = state.instance(&id).await?;
    let bk = state.backup_dir(&inst).await;
    blocking(move || {
        let backups = backup::folder_size(&bk);
        let install = backup::folder_size(&inst.install_dir);
        let inside = proc::is_within(&bk, &inst.install_dir);
        Ok(DiskUsage {
            install: if inside { install.saturating_sub(backups) } else { install },
            saves: backup::folder_size(&inst.saves_dir()),
            backups,
            logs: backup::folder_size(&inst.logs_dir()),
        })
    })
    .await
}
