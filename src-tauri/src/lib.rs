mod commands;
mod core;
mod error;
mod models;
mod paths;
mod state;

use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Helper-mode entry used by main.rs (`--send-ctrl-c <pid>`).
pub fn send_ctrl_c(pid: u32) -> i32 {
    core::proc::send_ctrl_c(pid)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch just focuses the running manager - two managers would fight over servers.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let loc = paths::resolve(app.handle())?;
            // The main window is created here (not from config) so a portable install can keep the
            // WebView2 profile inside its own folder instead of %LOCALAPPDATA%.
            let cfg = app.config().app.windows.iter().find(|w| w.label == "main").cloned().expect("main window config");
            let make = |data_dir: Option<std::path::PathBuf>| -> tauri::Result<tauri::WebviewWindow> {
                let mut builder = tauri::WebviewWindowBuilder::from_config(app.handle(), &cfg)?;
                if let Some(d) = data_dir {
                    builder = builder.data_directory(d);
                }
                // Debug builds only: expose the WebView over CDP for automated UI testing.
                #[cfg(debug_assertions)]
                if let Ok(port) = std::env::var("ESM_CDP_PORT") {
                    builder = builder.additional_browser_args(&format!(
                        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port={port}"
                    ));
                }
                builder.build()
            };
            let profile = loc.portable.then(|| loc.dir.join("webview"));
            // A WebView2 left over from a crashed/killed run keeps the profile locked: the next
            // window either hangs blank or fails with "resource is in use". Clear orphans first
            // (cheap process scan; WMI only when one is suspected), then fall back to a separate
            // recovery profile for this session – it only holds UI cache.
            if core::proc::orphan_webview_suspected() && core::proc::kill_orphaned_webviews() > 0 {
                std::thread::sleep(std::time::Duration::from_millis(600));
            }
            if let Err(e) = make(profile.clone()) {
                eprintln!("Window creation failed ({e}); retrying with a recovery WebView profile");
                core::proc::kill_orphaned_webviews();
                let base = match &profile {
                    Some(p) => p.clone(),
                    None => app.path().app_local_data_dir()?,
                };
                make(Some(base.join(format!("webview-recovery-{}", std::process::id()))))?;
            } else {
                // Normal start: drop recovery profiles from earlier sessions (best effort).
                let base = match &profile {
                    Some(p) => Some(p.clone()),
                    None => app.path().app_local_data_dir().ok(),
                };
                if let Some(Ok(rd)) = base.map(std::fs::read_dir) {
                    for e in rd.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("webview-recovery-")) {
                        let _ = std::fs::remove_dir_all(e.path());
                    }
                }
            }
            app.manage(Arc::new(state::AppState::load(loc.dir, loc.portable)));

            let title = MenuItem::with_id(app, "title", "Enshrouded Server Manager", false, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Open", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let start_all = MenuItem::with_id(app, "start-all", "Start all servers", true, None::<&str>)?;
            let stop_all = MenuItem::with_id(app, "stop-all", "Stop all servers (save first)", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit (servers keep running)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&title, &sep, &show, &start_all, &stop_all, &sep2, &quit])?;
            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().cloned().expect("icon"))
                .tooltip("Enshrouded Server Manager")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id().as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    "start-all" | "stop-all" => {
                        let start = e.id().as_ref() == "start-all";
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let st = app.state::<Arc<state::AppState>>().inner().clone();
                            let list = st.instances.read().await.clone();
                            for i in list.into_iter().filter(|i| i.is_installed()) {
                                let (_, main) = core::server::running_pids(&i);
                                let res = match (start, main.is_some()) {
                                    (true, false) => core::server::start(&st, &app, &i.id).await,
                                    (false, true) => core::server::stop(&st, &app, &i.id, false).await,
                                    _ => Ok(()),
                                };
                                if let Err(err) = res {
                                    state::notify(&app, &i.id, "error", format!("{}: {err}", i.name));
                                }
                            }
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            // Started by Windows at login with --minimized: stay in the tray, automation still runs.
            if !std::env::args().any(|a| a == "--minimized") {
                show_main(app.handle());
            }
            core::supervisor::spawn(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Window close (X, Alt+F4, taskbar): the UI decides - tray or quit.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.emit("app://close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_settings,
            commands::save_settings,
            commands::cancel_task,
            commands::test_webhook,
            commands::app_quit,
            commands::window_hide,
            commands::probe_folder,
            commands::list_instances,
            commands::get_statuses,
            commands::suggest_port,
            commands::create_instance,
            commands::update_instance,
            commands::validate_config,
            commands::reorder_instances,
            commands::delete_instance,
            commands::clone_instance,
            commands::find_existing_installs,
            commands::import_existing,
            commands::install_server,
            commands::check_update,
            commands::start_server,
            commands::stop_server,
            commands::restart_server,
            commands::cancel_deferred,
            commands::get_launch_line,
            commands::set_console_visible,
            commands::clear_crash_state,
            commands::query_server,
            commands::read_config,
            commands::write_config,
            commands::discard_pending_config,
            commands::list_config_history,
            commands::read_text_file,
            commands::list_bans,
            commands::remove_ban,
            commands::read_log_tail,
            commands::list_log_files,
            commands::list_worlds,
            commands::list_local_worlds,
            commands::scan_world_folder,
            commands::import_world,
            commands::restore_world_slot,
            commands::export_world,
            commands::delete_world,
            commands::reset_world,
            commands::metrics_history,
            commands::event_history,
            commands::player_sessions,
            commands::known_players,
            commands::build_history,
            commands::create_backup,
            commands::list_backups,
            commands::get_backup_dir,
            commands::restore_backup,
            commands::delete_backup,
            commands::verify_backup,
            commands::export_pack,
            commands::read_pack,
            commands::import_pack,
            commands::firewall_state,
            commands::add_firewall_rules,
            commands::remove_firewall_rules,
            commands::net_info,
            commands::steam_listing,
            commands::upnp_probe,
            commands::upnp_map,
            commands::upnp_unmap,
            commands::run_diagnostics,
            commands::disk_usage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Enshrouded Server Manager");
}
