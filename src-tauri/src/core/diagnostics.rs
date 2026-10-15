//! Health checks for one server, each with an optional one-click fix the UI can trigger.

use super::{config, network, proc, steamcmd};
use crate::models::*;
use crate::state::AppState;

fn check(id: &str, title: &str, level: &str, detail: impl Into<String>, fix: &str) -> DiagCheck {
    DiagCheck { id: id.into(), title: title.into(), level: level.into(), detail: detail.into(), fix: fix.into() }
}

pub async fn run(state: &AppState, inst: &ServerInstance) -> Vec<DiagCheck> {
    let mut out = vec![];
    let installed = inst.is_installed();

    // Files
    if installed {
        out.push(check("install", "Server files", "ok", format!("{}", inst.server_exe().display()), ""));
        match steamcmd::manifest_state_flags(&inst.install_dir) {
            Some(4) | None => {}
            Some(f) => out.push(check(
                "manifest",
                "Interrupted update",
                "warn",
                format!("Steam reports state flags {f} – a download was interrupted. Validate the files."),
                "validate",
            )),
        }
    } else {
        out.push(check("install", "Server files", "error", "The dedicated server isn't installed in this folder yet.", "install"));
    }

    // Config file on disk
    match config::read_doc(&inst.config_path()) {
        Ok(doc) if doc.is_empty() => out.push(check("configfile", "enshrouded_server.json", "info", "Not created yet – it's written on the first start.", "")),
        Ok(_) => out.push(check("configfile", "enshrouded_server.json", "ok", "The file parses as valid JSON.", "")),
        Err(e) => out.push(check("configfile", "enshrouded_server.json", "error", format!("{e} The server would stop with “Failed to parse Config”."), "config")),
    }

    // Settings & roles (the server's own validation rules)
    let issues = config::validate(&inst.config);
    let errors: Vec<&config::Issue> = issues.iter().filter(|i| i.level == "error").collect();
    let warns: Vec<&config::Issue> = issues.iter().filter(|i| i.level == "warn").collect();
    if !errors.is_empty() {
        out.push(check("roles", "Roles & settings", "error", errors.iter().map(|i| i.message.as_str()).collect::<Vec<_>>().join("\n"), "roles"));
    } else if !warns.is_empty() {
        out.push(check("roles", "Roles & settings", "warn", warns.iter().map(|i| i.message.as_str()).collect::<Vec<_>>().join("\n"), "roles"));
    } else {
        out.push(check("roles", "Roles & settings", "ok", format!("{} roles, every one password protected.", inst.config.user_groups.len()), ""));
    }
    let weak: Vec<&str> = inst
        .config
        .user_groups
        .iter()
        .filter(|g| {
            let p = g.password.to_lowercase();
            !p.is_empty() && (p == g.name.to_lowercase() || p == "password" || p == "123456" || p.len() < 6)
        })
        .map(|g| g.name.as_str())
        .collect();
    if !weak.is_empty() {
        out.push(check("weakpw", "Weak passwords", "warn", format!("Easy to guess: {}. Generate new ones under Roles.", weak.join(", ")), "roles"));
    }

    // Ports
    let instances = state.instances.read().await.clone();
    let clash: Vec<String> = instances
        .iter()
        .filter(|o| o.id != inst.id && o.udp_ports().iter().any(|p| inst.udp_ports().contains(p)))
        .map(|o| o.name.clone())
        .collect();
    if !clash.is_empty() {
        out.push(check("ports", "Port conflict", "error", format!("Query port {} is shared with: {}. Only one can run at a time.", inst.config.query_port, clash.join(", ")), "ports"));
    }
    let (pids, _) = super::server::running_pids(inst);
    let owners = proc::udp_port_owners().await;
    let busy: Vec<(u16, u32)> = inst.udp_ports().into_iter().filter_map(|p| owners.get(&p).filter(|o| !pids.contains(o)).map(|o| (p, *o))).collect();
    if let Some((port, pid)) = busy.first() {
        let mut sys = sysinfo::System::new();
        let name = proc::process_name(&mut sys, *pid).unwrap_or_default();
        out.push(check("portbusy", "Port in use", "error", format!("UDP {port} is held by {name} (PID {pid}). Pick another query port."), "ports"));
    } else if clash.is_empty() {
        out.push(check(
            "ports",
            "Query port",
            "ok",
            format!("UDP {} is available{}.", inst.config.query_port, if pids.is_empty() { "" } else { " (used by this server)" }),
            "",
        ));
    }

    // Firewall
    let fw = network::firewall_state(inst).await;
    if !fw.blocked_by.is_empty() {
        out.push(check(
            "firewall",
            "Windows Firewall is blocking the server",
            "error",
            format!("Inbound Block rule(s) “{}” exist for enshrouded_server.exe – usually from dismissing Windows’ “allow access?” prompt. Block beats Allow.", fw.blocked_by.join("”, “")),
            "firewall",
        ));
    } else if fw.ports_ok || fw.program_rule {
        out.push(check("firewall", "Windows Firewall", "ok", "An inbound rule allows the server.", ""));
    } else {
        out.push(check("firewall", "Windows Firewall", "warn", "No inbound rule for this server – players outside your PC may not reach it.", "firewall"));
    }

    // Memory (Keen: 16 GB for a full 16-player server; ~6 GB is plenty for a few friends)
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total_gb = sys.total_memory() as f64 / 1_073_741_824.0;
    let need = if inst.config.slot_count > 8 { 16.0 } else { 8.0 };
    if total_gb < need {
        out.push(check("ram", "Memory", "warn", format!("{need:.0} GB recommended for {} slots; this PC has {total_gb:.1} GB.", inst.config.slot_count), ""));
    } else {
        out.push(check("ram", "Memory", "ok", format!("{total_gb:.1} GB installed – {need:.0} GB recommended for {} slots.", inst.config.slot_count), ""));
    }

    // Disk
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let dir = proc::norm(&inst.install_dir);
    if let Some(d) = disks.iter().filter(|d| dir.starts_with(&proc::norm(d.mount_point()))).max_by_key(|d| d.mount_point().as_os_str().len()) {
        let free = d.available_space() as f64 / 1_073_741_824.0;
        let lvl = if free < 5.0 { "error" } else if free < 15.0 { "warn" } else { "ok" };
        out.push(check("disk", "Disk space", lvl, format!("{free:.1} GB free on {} (the server needs ~9 GB, updates temporarily more).", d.mount_point().display()), ""));
    }

    // Updates
    let (installed_b, latest_b, public_ip) = state.with_runtime(&inst.id, |r| (r.installed_build.clone(), r.latest_build.clone(), r.public_ip.clone())).await;
    if let (Some(i), Some(l)) = (&installed_b, &latest_b) {
        if i != l {
            out.push(check("update", "Server version", "warn", format!("Build {i} is installed but {l} is live – players on the new game version can't join until you update."), "update"));
        } else {
            out.push(check("update", "Server version", "ok", format!("Up to date (build {i})."), ""));
        }
    }

    // Steam master list – what the in-game Join list shows.
    if !pids.is_empty() {
        let ip = match public_ip {
            Some(ip) => Some(ip),
            None => network::public_ip(&state.http).await,
        };
        if let Some(ip) = ip {
            match network::steam_listing(&state.http, &ip).await {
                Ok(list) if list.iter().any(|s| s.gameport == inst.config.query_port) => {
                    out.push(check("listing", "Steam server list", "ok", format!("Listed as {ip}:{} – players can find “{}” in the Join menu.", inst.config.query_port, inst.config.name), ""))
                }
                Ok(_) => out.push(check(
                    "listing",
                    "Steam server list",
                    "warn",
                    format!("Steam has no server at {ip}:{} yet. It can take a minute after start; if it stays missing, check the firewall and router (Port Forwarding).", inst.config.query_port),
                    "ports",
                )),
                Err(e) => out.push(check("listing", "Steam server list", "info", format!("Couldn't ask Steam: {e}"), "")),
            }
        }
    }

    // Last log
    if let Ok(text) = std::fs::read_to_string(inst.log_path()) {
        let bad: Vec<&str> = text.lines().filter(|l| l.contains("Internal Error") || l.contains("Config Error") || l.starts_with("[F ")).take(3).collect();
        if !bad.is_empty() {
            out.push(check("log", "Errors in the last log", "error", bad.join("\n"), "log"));
        }
    }
    out
}
