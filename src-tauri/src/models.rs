//! Persisted and event types. Everything is camelCase on the wire and mirrored in `src/lib/types.ts`.
//!
//! Enshrouded's dedicated server reads one JSON file, `enshrouded_server.json`, next to its exe
//! (verified against build 23178631 / game SVN 1024233). There is no INI tree, no RCON and no
//! remote console: roles are `userGroups` with one password each, bans live in `bannedAccounts`
//! (written by the server itself when a host bans in-game) and live player info comes from the
//! log and the Steam A2S query on the query port.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// "Enshrouded Dedicated Server" – free Steam tool, anonymous SteamCMD login.
pub const SERVER_APP_ID: u32 = 2278520;
/// The game client (its saves and Steam Cloud folder use this id).
pub const GAME_APP_ID: u32 = 1203620;
pub const SERVER_EXE: &str = "enshrouded_server.exe";
pub const CONFIG_FILE: &str = "enshrouded_server.json";
pub const LOG_FILE: &str = "enshrouded_server.log";
/// The world container a dedicated server creates (`<id>`, `<id>-1` … rolling copies, `<id>-index`).
pub const SERVER_WORLD_ID: &str = "3ad85aea";
pub const DEFAULT_QUERY_PORT: u16 = 15637;
pub const MAX_SLOTS: u32 = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub steamcmd_dir: Option<PathBuf>,
    pub default_install_root: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub last_data_dir: Option<PathBuf>,
    /// Supervisor sample interval for live telemetry.
    pub telemetry_interval_ms: u64,
    pub confirm_destructive: bool,
    pub reduce_motion: bool,
    pub check_updates_on_launch: bool,
    pub accent: String,
    /// Closing the window keeps the manager (and its automation) running in the tray.
    pub close_to_tray: bool,
    /// Windows toast notifications for crashes, updates and players.
    pub desktop_notifications: bool,
    /// Masks IPs, passwords and Steam ids in the UI (for streaming / screenshots).
    pub streamer_mode: bool,
    /// Discord/Slack-compatible webhook for server events. Empty = off. Stored encrypted.
    pub webhook_url: String,
    /// Event levels sent to the webhook: success | error | warn | info
    pub webhook_levels: Vec<String>,
    /// Days of per-minute metrics kept in manager.db.
    pub metrics_retention_days: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            steamcmd_dir: None,
            default_install_root: None,
            backup_dir: None,
            last_data_dir: None,
            telemetry_interval_ms: 1000,
            confirm_destructive: true,
            reduce_motion: false,
            check_updates_on_launch: true,
            accent: "flame".into(),
            close_to_tray: true,
            desktop_notifications: true,
            streamer_mode: false,
            webhook_url: String::new(),
            webhook_levels: vec!["success".into(), "error".into()],
            metrics_retention_days: 30,
        }
    }
}

/// One entry of `userGroups`. Players type the group's password when they join and get its
/// permissions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct UserGroup {
    pub name: String,
    pub password: String,
    pub can_kick_ban: bool,
    pub can_access_inventories: bool,
    pub can_edit_world: bool,
    pub can_edit_base: bool,
    pub can_extend_base: bool,
    pub reserved_slots: u32,
}

impl Default for UserGroup {
    fn default() -> Self {
        Self {
            name: "Friend".into(),
            password: String::new(),
            can_kick_ban: false,
            can_access_inventories: true,
            can_edit_world: true,
            can_edit_base: true,
            can_extend_base: false,
            reserved_slots: 0,
        }
    }
}

impl UserGroup {
    /// The four groups a fresh server generates (same permissions as Keen's defaults).
    pub fn defaults(pw: impl Fn(&str) -> String) -> Vec<UserGroup> {
        let g = |name: &str, kick: bool, inv: bool, world: bool, base: bool, extend: bool| UserGroup {
            name: name.into(),
            password: pw(name),
            can_kick_ban: kick,
            can_access_inventories: inv,
            can_edit_world: world,
            can_edit_base: base,
            can_extend_base: extend,
            reserved_slots: 0,
        };
        vec![
            g("Admin", true, true, true, true, true),
            g("Friend", false, true, true, true, false),
            g("Guest", false, false, true, false, false),
            g("Visitor", false, false, false, false, false),
        ]
    }

    pub fn permissions(&self) -> [bool; 5] {
        [self.can_kick_ban, self.can_access_inventories, self.can_edit_world, self.can_edit_base, self.can_extend_base]
    }
}

/// The keys of `enshrouded_server.json` the profile owns. `bannedAccounts` and any key the
/// manager doesn't know stay owned by the file (the server writes bans itself).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerConfig {
    pub name: String,
    pub save_directory: String,
    pub log_directory: String,
    /// Bind address; "0.0.0.0" = all interfaces.
    pub ip: String,
    /// The one UDP port the server binds (Steam query + game traffic).
    pub query_port: u16,
    pub slot_count: u32,
    /// Server-browser filter tags (LookingForPlayers, English, BaseBuilding …).
    pub tags: Vec<String>,
    /// Proximity | Global
    pub voice_chat_mode: String,
    pub enable_voice_chat: bool,
    pub enable_text_chat: bool,
    /// Default | Relaxed | Hard | Survival | Custom
    pub game_settings_preset: String,
    /// Individual difficulty values – the server only applies them when the preset is "Custom".
    pub game_settings: BTreeMap<String, serde_json::Value>,
    pub user_groups: Vec<UserGroup>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "Enshrouded Server".into(),
            save_directory: "./savegame".into(),
            log_directory: "./logs".into(),
            ip: "0.0.0.0".into(),
            query_port: DEFAULT_QUERY_PORT,
            slot_count: 16,
            tags: vec![],
            voice_chat_mode: "Proximity".into(),
            enable_voice_chat: false,
            enable_text_chat: false,
            game_settings_preset: "Default".into(),
            game_settings: BTreeMap::new(),
            user_groups: UserGroup::defaults(|_| String::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchSettings {
    /// Ask the router to forward the query port over UPnP on every start.
    pub upnp: bool,
    /// Keep the server's console window visible (hidden by default).
    pub show_console: bool,
    /// normal | belowNormal | aboveNormal | high | idle
    pub priority: String,
    /// CPU cores the server may use. Empty = all cores.
    pub affinity: Vec<u32>,
    /// Free-form extra arguments appended to the command line.
    pub extra_args: String,
}

impl Default for LaunchSettings {
    fn default() -> Self {
        Self { upnp: false, show_console: false, priority: "normal".into(), affinity: vec![], extra_args: String::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ScheduleEntry {
    pub enabled: bool,
    /// Local time, "HH:MM".
    pub time: String,
    /// 0 = Sunday … 6 = Saturday. Empty = every day.
    pub days: Vec<u8>,
    /// restart | backup | update | stop | start
    pub action: String,
}

impl Default for ScheduleEntry {
    fn default() -> Self {
        Self { enabled: true, time: "04:00".into(), days: vec![], action: "restart".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AutomationSettings {
    pub auto_start: bool,
    pub auto_restart: bool,
    pub restart_delay_secs: u32,
    /// Crash-loop guard: stop auto-restarting after this many crashes within 15 minutes.
    pub crash_limit: u32,
    pub stop_timeout_secs: u32,
    pub schedules: Vec<ScheduleEntry>,
    pub backup_interval_mins: u32,
    pub backup_keep: u32,
    pub backup_on_stop: bool,
    pub backup_before_update: bool,
    pub update_on_start: bool,
    pub update_check_mins: u32,
    pub auto_update: bool,
    /// Wait for the server to be empty before an automatic update/restart (up to 60 minutes).
    pub wait_for_empty: bool,
    /// Restart when the server's memory exceeds this many MB (0 = off).
    pub memory_limit_mb: u32,
    /// Restart when the server has been up this many hours (0 = off).
    pub max_uptime_hours: u32,
    /// Stop the server after this many minutes with nobody online (0 = off).
    pub idle_stop_mins: u32,
}

impl Default for AutomationSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            auto_restart: true,
            restart_delay_secs: 10,
            crash_limit: 3,
            stop_timeout_secs: 60,
            schedules: vec![],
            backup_interval_mins: 60,
            backup_keep: 24,
            backup_on_stop: true,
            backup_before_update: true,
            update_on_start: false,
            update_check_mins: 60,
            auto_update: false,
            wait_for_empty: true,
            memory_limit_mb: 0,
            max_uptime_hours: 0,
            idle_stop_mins: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerInstance {
    pub id: String,
    pub name: String,
    pub color: String,
    pub install_dir: PathBuf,
    pub created_at: i64,
    pub notes: String,
    /// Steam branch ("public" or a beta name) and its password.
    pub branch: String,
    pub branch_password: String,
    pub config: ServerConfig,
    pub launch: LaunchSettings,
    pub automation: AutomationSettings,
}

impl Default for ServerInstance {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Enshrouded Server".into(),
            color: "flame".into(),
            install_dir: PathBuf::new(),
            created_at: 0,
            notes: String::new(),
            branch: "public".into(),
            branch_password: String::new(),
            config: ServerConfig::default(),
            launch: LaunchSettings::default(),
            automation: AutomationSettings::default(),
        }
    }
}

/// A `./relative` or absolute folder from the config, resolved against the install folder.
fn resolve_dir(base: &Path, v: &str, fallback: &str) -> PathBuf {
    let v = v.trim();
    let v = if v.is_empty() { fallback } else { v };
    let p = PathBuf::from(v.replace('/', "\\"));
    if p.is_absolute() {
        p
    } else {
        base.join(v.trim_start_matches("./").trim_start_matches(".\\").replace('/', "\\"))
    }
}

impl ServerInstance {
    /// Every UDP port the server binds and players need: just the query port since 0.9.
    pub fn udp_ports(&self) -> Vec<u16> {
        vec![self.config.query_port]
    }
    pub fn config_path(&self) -> PathBuf {
        self.install_dir.join(CONFIG_FILE)
    }
    pub fn saves_dir(&self) -> PathBuf {
        resolve_dir(&self.install_dir, &self.config.save_directory, "./savegame")
    }
    pub fn logs_dir(&self) -> PathBuf {
        resolve_dir(&self.install_dir, &self.config.log_directory, "./logs")
    }
    pub fn log_path(&self) -> PathBuf {
        self.logs_dir().join(LOG_FILE)
    }
    pub fn server_exe(&self) -> PathBuf {
        resolve_server_exe(&self.install_dir)
    }
    pub fn is_installed(&self) -> bool {
        self.server_exe().exists()
    }
}

pub fn resolve_server_exe(install_dir: &Path) -> PathBuf {
    install_dir.join(SERVER_EXE)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ServerState {
    NotInstalled,
    Stopped,
    Installing,
    Updating,
    Starting,
    Running,
    Stopping,
    Restarting,
    Crashed,
}

impl ServerState {
    pub fn is_live(self) -> bool {
        matches!(self, ServerState::Starting | ServerState::Running | ServerState::Stopping)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub id: String,
    pub state: ServerState,
    pub pid: Option<u32>,
    pub started_at: Option<i64>,
    pub ready_at: Option<i64>,
    pub installed_build: Option<String>,
    pub latest_build: Option<String>,
    /// Game build from the log (`Game Version (SVN): 1024233`).
    pub game_version: Option<String>,
    /// Version the server answers A2S with (`0.0.15.0`).
    pub query_version: Option<String>,
    pub crash_count: u32,
    pub crash_loop: bool,
    pub pending_restart: bool,
    /// Players online (names from the log, count confirmed by A2S).
    pub players: Vec<OnlinePlayer>,
    /// Player count reported by the server over A2S (None = no answer yet).
    pub query_players: Option<u32>,
    pub next_schedule: Option<NextSchedule>,
    pub last_backup: Option<i64>,
    pub last_exit_code: Option<i64>,
    pub busy: Option<String>,
    /// Steam game-server id from the log.
    pub steam_id: Option<String>,
    /// Public IPv4 the server registered with Steam.
    pub public_ip: Option<String>,
    /// The last world save the log reported.
    pub last_save: Option<i64>,
    /// An automatic restart/update/stop waiting for players to leave.
    pub waiting: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OnlinePlayer {
    pub name: String,
    pub since: i64,
    /// Permission line from the log ("Can Kick Ban, …") – which role the password unlocked.
    pub permissions: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextSchedule {
    pub at: i64,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcSample {
    pub id: String,
    pub cpu: f32,
    pub mem: u64,
    pub virt: u64,
    pub read_bps: u64,
    pub write_bps: u64,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemSample {
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub rx_bps: u64,
    pub tx_bps: u64,
    pub cores: usize,
    pub per_core: Vec<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEvent {
    pub ts: i64,
    pub system: SystemSample,
    pub servers: Vec<ProcSample>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub task_id: String,
    pub instance_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub message: String,
    pub progress: Option<f64>,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogBatch {
    pub id: String,
    /// server | manager | steamcmd
    pub source: String,
    pub lines: Vec<String>,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEvent {
    pub id: String,
    /// info | success | warn | error
    pub level: String,
    pub message: String,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEvent {
    pub id: String,
    pub name: String,
    /// join | leave
    pub kind: String,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub created_at: i64,
    pub label: String,
    pub kind: String,
    pub files: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    pub path: PathBuf,
    pub exists: bool,
    pub text: String,
    pub modified: Option<i64>,
    /// Text saved while the server ran – written once it exits.
    pub pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    pub format: u32,
    pub app_version: String,
    pub exported_at: i64,
    pub instance: ServerInstance,
    pub has_saves: bool,
    pub has_config: bool,
    pub includes_secrets: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagCheck {
    pub id: String,
    pub title: String,
    /// ok | warn | error | info
    pub level: String,
    pub detail: String,
    /// Fix the UI can trigger: firewall | install | validate | settings | roles | ports | update | log | ""
    pub fix: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: PathBuf,
    pub portable: bool,
    pub app_id: u32,
    pub cores: usize,
    pub total_memory: u64,
    pub os: String,
    pub hostname: String,
    /// Secrets that couldn't be decrypted at load (portable folder moved to another PC/account).
    pub lost_secrets: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_resolve_relative_and_absolute() {
        let mut i = ServerInstance { install_dir: PathBuf::from(r"D:\srv\a"), ..Default::default() };
        assert_eq!(i.saves_dir(), PathBuf::from(r"D:\srv\a\savegame"));
        assert_eq!(i.log_path(), PathBuf::from(r"D:\srv\a\logs\enshrouded_server.log"));
        i.config.save_directory = r"E:\worlds\one".into();
        assert_eq!(i.saves_dir(), PathBuf::from(r"E:\worlds\one"));
        i.config.save_directory = "".into();
        assert_eq!(i.saves_dir(), PathBuf::from(r"D:\srv\a\savegame"));
    }
}
