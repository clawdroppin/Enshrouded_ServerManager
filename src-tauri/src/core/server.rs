//! Server lifecycle: launch line, config sync, start / stop / restart / update.
//!
//! Every lifecycle operation for one server runs under `AppState::op_lock(id)` so a scheduled
//! restart, a crash restart and a button click can never interleave.

use super::{backup, config, proc, steamcmd};
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::{manager_log, notify, notify_kind, now_ms, AppState, TaskReporter};
use std::time::Duration;
use tauri::AppHandle;

/// Split a free-form argument string, keeping "quoted values" together.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut in_q = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                cur.push(c);
            }
            c if c.is_whitespace() && !in_q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Command-line arguments after the exe. The server takes everything from
/// enshrouded_server.json, so this is only what the user adds (passwords never go here – the
/// command line is visible to every process on the PC).
pub fn build_args(inst: &ServerInstance) -> Vec<String> {
    split_args(&inst.launch.extra_args)
}

pub fn launch_line(inst: &ServerInstance) -> String {
    let args = build_args(inst);
    if args.is_empty() {
        format!("\"{}\"", inst.server_exe().display())
    } else {
        format!("\"{}\" {}", inst.server_exe().display(), args.join(" "))
    }
}

/// PIDs of this server's running processes and the main PID.
pub fn running_pids(inst: &ServerInstance) -> (Vec<u32>, Option<u32>) {
    let mut sys = sysinfo::System::new();
    let all = proc::scan_servers(&mut sys);
    let mine = proc::procs_for_install(&all, &inst.install_dir);
    (mine.iter().map(|p| p.pid).collect(), mine.first().map(|p| p.pid))
}

/// Write a raw config edit made while the server was running, then pull the profile-owned keys
/// back into the profile. Returns true when something was applied.
pub async fn apply_pending_raw(state: &AppState, inst: &ServerInstance) -> AppResult<bool> {
    let Some(text) = state.with_runtime(&inst.id, |r| r.pending_raw.take()).await else { return Ok(false) };
    config::write_text(&inst.config_path(), &text)?;
    let mut updated = inst.clone();
    config::read_file_into(&inst.config_path(), &mut updated.config)?;
    state.put_instance(&updated).await?;
    Ok(true)
}

pub fn validate_for_start(inst: &ServerInstance) -> AppResult<Vec<String>> {
    config::ensure_valid(&inst.config)?;
    Ok(config::validate(&inst.config).into_iter().filter(|i| i.level == "warn").map(|i| i.message).collect())
}

pub async fn start(state: &AppState, app: &AppHandle, id: &str) -> AppResult<()> {
    let lock = state.op_lock(id).await;
    let _g = lock.lock().await;
    start_locked(state, app, id).await
}

async fn start_locked(state: &AppState, app: &AppHandle, id: &str) -> AppResult<()> {
    let inst = state.instance(id).await?;
    if !inst.is_installed() {
        return Err(AppError::msg("Server files are not installed yet. Click Install first."));
    }
    if let (_, Some(pid)) = running_pids(&inst) {
        state
            .with_runtime(id, |r| {
                r.pid = Some(pid);
                r.state = Some(ServerState::Running);
                r.started_at.get_or_insert(now_ms());
            })
            .await;
        state.emit_status(app, id).await;
        return Ok(());
    }
    if apply_pending_raw(state, &inst).await? {
        manager_log(app, id, "Applied config file edits saved while the server was running.");
    }
    let inst = state.instance(id).await?;
    for w in validate_for_start(&inst)? {
        manager_log(app, id, format!("Note: {w}"));
    }

    // Another program (or another server) on the query port.
    let owners = proc::udp_port_owners().await;
    for port in inst.udp_ports() {
        if let Some(pid) = owners.get(&port) {
            let mut sys = sysinfo::System::new();
            let name = proc::process_name(&mut sys, *pid).unwrap_or_else(|| "another program".into());
            return Err(AppError::msg(format!(
                "UDP port {port} is already in use by {name} (PID {pid}). Pick a different query port under Server Settings."
            )));
        }
    }

    if inst.automation.update_on_start {
        state.set_busy(app, id, Some("Checking for updates")).await;
        let res = update_files(state, app, &inst, false).await;
        state.set_busy(app, id, None).await;
        if let Err(e) = res {
            notify(app, id, "warn", format!("Update before start failed, starting anyway: {e}"));
        }
    }

    config::sync(&inst)?;
    std::fs::create_dir_all(inst.saves_dir()).ok();
    let args = build_args(&inst);
    let exe = inst.server_exe();
    // The server moves the old log into logs\backup when it starts: remember the current file so
    // the tail skips it and starts from the top of the new one.
    let old_len = std::fs::metadata(inst.log_path()).map(|m| m.len()).unwrap_or(0);
    let old_sig = super::supervisor::log_signature(&inst.log_path());
    let pid = proc::spawn_server(&exe, &args, !inst.launch.show_console, &inst.launch.priority)?;
    if !inst.launch.affinity.is_empty() {
        let _ = proc::apply_priority_affinity(pid, &inst.launch.priority, &inst.launch.affinity);
    }
    state
        .with_runtime(id, |r| {
            r.pid = Some(pid);
            r.state = Some(ServerState::Starting);
            r.started_at = Some(now_ms());
            r.ready_at = None;
            r.intentional_stop = false;
            r.pending_restart = false;
            r.restart_at = None;
            r.online.clear();
            r.query_players = None;
            r.log_offset = old_len;
            r.log_partial.clear();
            r.log_sig = old_sig;
            r.console_hidden = !inst.launch.show_console;
            r.log_ready = false;
            r.fatal = None;
            r.game_version = None;
            r.query_version = None;
            r.steam_id = None;
            r.public_ip = None;
            r.last_exit_code = None;
            r.empty_since = None;
            r.waiting_empty = None;
        })
        .await;
    manager_log(app, id, format!("Launching {} (PID {pid}) {}", exe.file_name().unwrap_or_default().to_string_lossy(), args.join(" ")));
    state.history.event(Some(id), "info", "lifecycle", "Server starting");
    if inst.launch.upnp {
        let (app2, i2) = (app.clone(), inst.clone());
        tauri::async_runtime::spawn(async move {
            match super::upnp::map_server(&i2).await {
                Ok(lines) => manager_log(&app2, &i2.id, format!("UPnP: {}", lines.join(", "))),
                Err(e) => notify(&app2, &i2.id, "warn", format!("UPnP port forwarding failed: {e}")),
            }
        });
    }
    state.emit_status(app, id).await;
    Ok(())
}

/// Graceful stop: Ctrl+Break to the server console (it saves the world and exits – verified:
/// "Trigger gameflow shutdown, exit: Ctrl_Break" … "[server] Saved"), then force-kill on timeout.
pub async fn stop(state: &AppState, app: &AppHandle, id: &str, force: bool) -> AppResult<()> {
    let lock = state.op_lock(id).await;
    let _g = lock.lock().await;
    stop_locked(state, app, id, force).await
}

async fn stop_locked(state: &AppState, app: &AppHandle, id: &str, force: bool) -> AppResult<()> {
    let inst = state.instance(id).await?;
    let (pids, main) = running_pids(&inst);
    let Some(main) = main else {
        state
            .with_runtime(id, |r| {
                r.pid = None;
                r.state = Some(ServerState::Stopped);
                r.restart_at = None;
                r.online.clear();
            })
            .await;
        state.emit_status(app, id).await;
        return Ok(());
    };
    state
        .with_runtime(id, |r| {
            r.intentional_stop = true;
            r.restart_at = None;
            r.state = Some(ServerState::Stopping);
        })
        .await;
    state.emit_status(app, id).await;

    let timeout = inst.automation.stop_timeout_secs.clamp(5, 900) as u64;
    let mut exited = false;
    if !force {
        manager_log(app, id, "Asking the server to save and shut down…");
        if proc::request_graceful_exit(main).await {
            let mut sys = sysinfo::System::new();
            for _ in 0..timeout * 2 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if !pids.iter().any(|p| proc::pid_alive(&mut sys, *p)) {
                    exited = true;
                    break;
                }
            }
            if !exited {
                manager_log(app, id, format!("Server did not exit within {timeout}s – forcing it to close."));
                notify(app, id, "warn", format!("{} didn't shut down within {timeout}s and was force-closed.", inst.name));
            }
        } else {
            manager_log(app, id, "Couldn't reach the server console – forcing it to close.");
        }
    }
    if !exited {
        for p in &pids {
            let _ = proc::kill_tree(*p).await;
        }
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    state
        .with_runtime(id, |r| {
            r.pid = None;
            r.state = Some(ServerState::Stopped);
            r.started_at = None;
            r.ready_at = None;
            r.online.clear();
            r.query_players = None;
            r.pending_restart = false;
            r.waiting_empty = None;
        })
        .await;
    state.history.sessions_close_all(id, now_ms());
    manager_log(app, id, if exited { "Server saved and stopped." } else { "Server process terminated." });
    notify_kind(app, id, "info", "lifecycle", if exited { format!("{} stopped", inst.name) } else { format!("{} was force-stopped", inst.name) });
    if apply_pending_raw(state, &inst).await.unwrap_or(false) {
        manager_log(app, id, "Applied config file edits saved while the server was running.");
        let _ = tauri::Emitter::emit(app, "instances-changed", ());
    }
    state.emit_status(app, id).await;

    if inst.automation.backup_on_stop && inst.saves_dir().exists() {
        state.set_busy(app, id, Some("Backing up")).await;
        let res = backup::create(state, &inst, "After stop", "auto").await;
        state.set_busy(app, id, None).await;
        match res {
            Ok(b) => {
                state.with_runtime(id, |r| r.last_backup = b.created_at).await;
                let _ = tauri::Emitter::emit(app, "backup-created", &b);
            }
            Err(e) => notify(app, id, "warn", format!("Backup after stop failed: {e}")),
        }
    }
    Ok(())
}

pub async fn restart(state: &AppState, app: &AppHandle, id: &str, update: bool) -> AppResult<()> {
    let lock = state.op_lock(id).await;
    let _g = lock.lock().await;
    let inst = state.instance(id).await?;
    state
        .with_runtime(id, |r| {
            r.state = Some(ServerState::Restarting);
            r.crash_loop = false;
        })
        .await;
    state.emit_status(app, id).await;
    stop_locked(state, app, id, false).await?;
    if update {
        state.with_runtime(id, |r| r.state = Some(ServerState::Updating)).await;
        state.emit_status(app, id).await;
        let res = update_files(state, app, &inst, false).await;
        state.with_runtime(id, |r| r.state = Some(ServerState::Stopped)).await;
        if let Err(e) = res {
            notify(app, id, "error", format!("Update failed, restarting on the current build: {e}"));
        }
    }
    start_locked(state, app, id).await
}

/// SteamCMD install/update of one server's files (caller decides about stopping first).
pub async fn update_files(state: &AppState, app: &AppHandle, inst: &ServerInstance, validate: bool) -> AppResult<()> {
    let title = if !inst.is_installed() {
        format!("Installing {}", inst.name)
    } else if validate {
        format!("Validating {}", inst.name)
    } else {
        format!("Updating {}", inst.name)
    };
    let r = TaskReporter::new(app, "steamcmd", &title, Some(&inst.id));
    // SteamCMD stages a full copy during installs and big updates; fail early with a clear
    // message instead of half way through the download.
    let need: u64 = if inst.is_installed() { 4 } else { 12 } * 1_073_741_824;
    if let Some(free) = proc::free_space(&inst.install_dir) {
        if free < need {
            let msg = format!(
                "Only {:.1} GB free on the drive of {} – {} needs about {} GB. Free some space or move the server under Install & Files.",
                free as f64 / 1_073_741_824.0,
                inst.install_dir.display(),
                if inst.is_installed() { "an update" } else { "the install" },
                need / 1_073_741_824
            );
            r.fail(&msg);
            return Err(AppError::msg(msg));
        }
    }
    let cancel = state.register_cancel(&r.task_id).await;
    let steam_dir = state.steamcmd_dir().await;
    let before = steamcmd::local_build(&inst.install_dir);
    let res = {
        let _steam = state.steam_lock.lock().await;
        steamcmd::app_update(&state.http, &steam_dir, &inst.install_dir, &inst.branch, &inst.branch_password, validate, &r, cancel).await
    };
    state.cancels.lock().await.remove(&r.task_id);
    match res {
        Ok(()) => {
            let build = steamcmd::local_build(&inst.install_dir);
            state.with_runtime(&inst.id, |rt| rt.installed_build = build.clone()).await;
            if let Some(b) = &build {
                state.history.build(&inst.id, b, &inst.branch);
            }
            // Write the profile's config so settings can be edited before the first start.
            let _ = config::sync(inst);
            let b = build.clone().unwrap_or_else(|| "?".into());
            r.done(&format!("{} – build {b}", title.replace("ing ", "ed ")));
            if before.is_some() && before != build {
                state.history.event(Some(&inst.id), "success", "update", &format!("Updated to build {b}"));
            }
            Ok(())
        }
        Err(e) => {
            r.fail(&e.to_string());
            Err(e)
        }
    }
}

/// Full update job: stops the server if needed (with a backup), updates, restarts if it was up.
pub async fn update_job(state: &AppState, app: &AppHandle, id: &str, validate: bool) -> AppResult<()> {
    let lock = state.op_lock(id).await;
    let _g = lock.lock().await;
    let inst = state.instance(id).await?;
    let (_, main) = running_pids(&inst);
    let was_running = main.is_some();
    if was_running {
        manager_log(app, id, "Stopping the server to update it…");
        stop_locked(state, app, id, false).await?;
    }
    // stop_locked already backs up when "backup on stop" is on; otherwise honour "before update".
    if inst.automation.backup_before_update && !(was_running && inst.automation.backup_on_stop) && inst.saves_dir().exists() {
        if let Ok(b) = backup::create(state, &inst, "Before update", "auto").await {
            state.with_runtime(id, |r| r.last_backup = b.created_at).await;
        }
    }
    state.with_runtime(id, |r| r.state = Some(if inst.is_installed() { ServerState::Updating } else { ServerState::Installing })).await;
    state.emit_status(app, id).await;
    let res = update_files(state, app, &inst, validate).await;
    state.with_runtime(id, |r| r.state = None).await;
    state.emit_status(app, id).await;
    res?;
    if was_running {
        start_locked(state, app, id).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_are_only_the_extras() {
        let mut inst = ServerInstance::default();
        assert!(build_args(&inst).is_empty());
        inst.launch.extra_args = "--foo \"--bar=a b\"".into();
        assert_eq!(build_args(&inst), vec!["--foo".to_string(), "\"--bar=a b\"".to_string()]);
    }

    #[test]
    fn sync_writes_profile_and_keeps_bans() {
        let dir = std::env::temp_dir().join(format!("esm-sync-{}", uuid::Uuid::new_v4()));
        let mut inst = ServerInstance { install_dir: dir.clone(), ..Default::default() };
        inst.config.name = "My Server".into();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(inst.config_path(), "{\n\t\"name\": \"Old\",\n\t\"bannedAccounts\": [ { \"displayName\": \"X\" } ]\n}\n").unwrap();
        config::sync(&inst).unwrap();
        let text = std::fs::read_to_string(inst.config_path()).unwrap();
        assert!(text.contains("\"name\": \"My Server\""));
        assert!(text.contains("\"displayName\": \"X\""));
        let mut back = ServerConfig::default();
        config::read_file_into(&inst.config_path(), &mut back).unwrap();
        assert_eq!(back.name, "My Server");
        let _ = std::fs::remove_dir_all(dir);
    }
}
