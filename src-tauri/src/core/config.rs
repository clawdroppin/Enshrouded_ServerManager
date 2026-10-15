//! `enshrouded_server.json`: lossless read-modify-write and validation.
//!
//! The file is shared with the server: it adds missing keys and random passwords on first start
//! and writes `bannedAccounts` when a host bans someone in-game. So the manager never writes a
//! whole file from the profile – it loads the current document, replaces only the keys the profile
//! owns and keeps everything else (bans, keys added by future game versions) in their original
//! order. Output uses tabs like the server's own writer.

use crate::error::{AppError, AppResult};
use crate::models::*;
use serde::Serialize;
use serde_json::{Map, Value};
use std::path::Path;

pub const PRESETS: &[&str] = &["Default", "Relaxed", "Hard", "Survival", "Custom"];

/// Read the file as an ordered JSON object (empty object when missing).
pub fn read_doc(path: &Path) -> AppResult<Map<String, Value>> {
    let text = match std::fs::read(path) {
        Ok(b) => String::from_utf8_lossy(&b).trim_start_matches('\u{feff}').to_string(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => return Err(e.into()),
    };
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => Err(AppError::msg(format!("{} isn't a JSON object.", path.display()))),
        Err(e) => Err(AppError::msg(format!("{} has a JSON error at line {}, column {}: {e}", path.display(), e.line(), e.column()))),
    }
}

/// Serialize the way the server does: tab indentation, CRLF-free, trailing newline.
pub fn to_text(doc: &Map<String, Value>) -> String {
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    Value::Object(doc.clone()).serialize(&mut ser).expect("json serialize");
    let mut s = String::from_utf8(buf).unwrap_or_default();
    s.push('\n');
    s
}

/// Write with a rolling history (`_config-history/enshrouded_server.<stamp>.json`, last 30 kept).
pub fn write_text(path: &Path, text: &str) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
        if let Ok(old) = std::fs::read(path) {
            if !old.is_empty() && old != text.as_bytes() {
                let hist = history_dir(path);
                std::fs::create_dir_all(&hist)?;
                let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%.3f");
                let _ = std::fs::write(hist.join(format!("enshrouded_server.{stamp}.json")), &old);
                prune_history(&hist, 30);
            }
        }
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn history_dir(path: &Path) -> std::path::PathBuf {
    path.parent().unwrap_or(Path::new(".")).join("_config-history")
}

fn prune_history(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    v.sort();
    let n = v.len();
    for p in v.into_iter().take(n.saturating_sub(keep)) {
        let _ = std::fs::remove_file(p);
    }
}

/// Whole numbers go out as integers (`1`, not `1.0`) – the server writes them that way and the
/// nanosecond durations must stay integers.
fn num(v: &Value) -> Value {
    match v.as_f64() {
        Some(f) if v.is_f64() && f.fract() == 0.0 && f.abs() < 9.0e15 => Value::from(f as i64),
        _ => v.clone(),
    }
}

fn group_value(g: &UserGroup) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), g.name.trim().into());
    m.insert("password".into(), g.password.clone().into());
    m.insert("canKickBan".into(), g.can_kick_ban.into());
    m.insert("canAccessInventories".into(), g.can_access_inventories.into());
    m.insert("canEditWorld".into(), g.can_edit_world.into());
    m.insert("canEditBase".into(), g.can_edit_base.into());
    m.insert("canExtendBase".into(), g.can_extend_base.into());
    m.insert("reservedSlots".into(), g.reserved_slots.into());
    Value::Object(m)
}

/// Put the profile-owned keys into a document, keeping key order and all other keys.
pub fn apply(doc: &mut Map<String, Value>, c: &ServerConfig) {
    let mut set = |k: &str, v: Value| {
        if let Some(slot) = doc.get_mut(k) {
            *slot = v;
        } else {
            doc.insert(k.to_string(), v);
        }
    };
    set("name", c.name.trim().into());
    set("saveDirectory", c.save_directory.trim().into());
    set("logDirectory", c.log_directory.trim().into());
    set("ip", if c.ip.trim().is_empty() { "0.0.0.0".into() } else { c.ip.trim().into() });
    set("queryPort", c.query_port.into());
    set("slotCount", c.slot_count.clamp(1, MAX_SLOTS).into());
    set("tags", Value::Array(c.tags.iter().map(|t| Value::from(t.as_str())).collect()));
    set("voiceChatMode", c.voice_chat_mode.clone().into());
    set("enableVoiceChat", c.enable_voice_chat.into());
    set("enableTextChat", c.enable_text_chat.into());
    set("gameSettingsPreset", c.game_settings_preset.clone().into());
    // Keep values the file has for settings the profile doesn't know (newer game versions).
    let mut gs = doc.get("gameSettings").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    for (k, v) in &c.game_settings {
        gs.insert(k.clone(), num(v));
    }
    doc.insert("gameSettings".into(), Value::Object(gs));
    doc.insert("userGroups".into(), Value::Array(c.user_groups.iter().map(group_value).collect()));
    if !doc.contains_key("bannedAccounts") {
        doc.insert("bannedAccounts".into(), Value::Array(vec![]));
    }
}

/// Read the profile-owned keys out of a document. Keys missing from the file keep the profile's value.
pub fn read_into(doc: &Map<String, Value>, c: &mut ServerConfig) {
    let s = |k: &str| doc.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let b = |k: &str| doc.get(k).and_then(|v| v.as_bool());
    if let Some(v) = s("name") {
        c.name = v;
    }
    if let Some(v) = s("saveDirectory") {
        c.save_directory = v;
    }
    if let Some(v) = s("logDirectory") {
        c.log_directory = v;
    }
    if let Some(v) = s("ip") {
        c.ip = v;
    }
    if let Some(v) = doc.get("queryPort").and_then(|v| v.as_u64()) {
        c.query_port = v.min(65535) as u16;
    }
    if let Some(v) = doc.get("slotCount").and_then(|v| v.as_u64()) {
        c.slot_count = v as u32;
    }
    if let Some(Value::Array(a)) = doc.get("tags") {
        c.tags = a.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect();
    }
    if let Some(v) = s("voiceChatMode") {
        c.voice_chat_mode = v;
    }
    if let Some(v) = b("enableVoiceChat") {
        c.enable_voice_chat = v;
    }
    if let Some(v) = b("enableTextChat") {
        c.enable_text_chat = v;
    }
    if let Some(v) = s("gameSettingsPreset") {
        c.game_settings_preset = v;
    }
    if let Some(Value::Object(m)) = doc.get("gameSettings") {
        c.game_settings = m.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    }
    if let Some(Value::Array(a)) = doc.get("userGroups") {
        c.user_groups = a.iter().filter_map(|g| serde_json::from_value::<UserGroup>(g.clone()).ok()).collect();
    }
}

pub fn read_file_into(path: &Path, c: &mut ServerConfig) -> AppResult<bool> {
    if !path.exists() {
        return Ok(false);
    }
    read_into(&read_doc(path)?, c);
    Ok(true)
}

/// Write the profile into the config file (creating it when missing).
pub fn sync(inst: &ServerInstance) -> AppResult<()> {
    let path = inst.config_path();
    let mut doc = read_doc(&path)?;
    apply(&mut doc, &inst.config);
    let text = to_text(&doc);
    if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
        write_text(&path, &text)?;
    }
    Ok(())
}

// --------------------------------------------------------------------------------------- bans

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BanEntry {
    /// Index in `bannedAccounts` when read (entries are matched by their raw JSON on removal).
    pub raw: String,
    pub account_id_hash: String,
    pub display_name: String,
    pub character_name: String,
    pub ban_date: Option<i64>,
}

fn field_str(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        // `accountIdHash` may be written as { "value": … } like other wrapped fields.
        Some(Value::Object(o)) => o.get("value").map(|x| x.to_string().trim_matches('"').to_string()).unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn read_bans(path: &Path) -> AppResult<Vec<BanEntry>> {
    let doc = read_doc(path)?;
    let Some(Value::Array(a)) = doc.get("bannedAccounts") else { return Ok(vec![]) };
    Ok(a.iter()
        .map(|v| BanEntry {
            raw: v.to_string(),
            account_id_hash: field_str(v, "accountIdHash"),
            display_name: field_str(v, "displayName"),
            character_name: field_str(v, "characterName"),
            ban_date: v.get("banDate").and_then(|d| d.as_i64().or_else(|| d.get("value").and_then(|x| x.as_i64()))),
        })
        .collect())
}

/// Remove one ban (matched by its raw JSON). Returns true when something was removed.
pub fn remove_ban(path: &Path, raw: &str) -> AppResult<bool> {
    let mut doc = read_doc(path)?;
    let Some(Value::Array(a)) = doc.get_mut("bannedAccounts") else { return Ok(false) };
    let before = a.len();
    a.retain(|v| v.to_string() != raw);
    let removed = a.len() != before;
    if removed {
        write_text(path, &to_text(&doc))?;
    }
    Ok(removed)
}

// ---------------------------------------------------------------------------------- validation

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// error = the server refuses the file; warn = allowed but probably unintended.
    pub level: String,
    pub message: String,
}

fn issue(level: &str, m: impl Into<String>) -> Issue {
    Issue { level: level.into(), message: m.into() }
}

/// The checks the server itself performs on load (its "Internal Error: …" messages, read from the
/// binary), plus a few plain-sense warnings. An error here means the server would refuse to start.
pub fn validate(c: &ServerConfig) -> Vec<Issue> {
    let mut out = vec![];
    if c.name.trim().is_empty() {
        out.push(issue("error", "The server needs a name."));
    }
    if c.ip.trim() != "0.0.0.0" && !c.ip.trim().is_empty() && c.ip.trim().parse::<std::net::Ipv4Addr>().is_err() {
        out.push(issue("error", format!("“{}” isn't a valid IPv4 address (Config Error: Failed to parse Ip).", c.ip.trim())));
    }
    if c.query_port < 1024 {
        out.push(issue("error", "Pick a query port of 1024 or higher."));
    }
    if !(1..=MAX_SLOTS).contains(&c.slot_count) {
        out.push(issue("error", format!("Slot count must be between 1 and {MAX_SLOTS}.")));
    }
    if !PRESETS.contains(&c.game_settings_preset.as_str()) {
        out.push(issue("error", format!("Unknown difficulty preset “{}”.", c.game_settings_preset)));
    }
    let groups = &c.user_groups;
    if groups.is_empty() {
        out.push(issue("warn", "There are no roles – nobody can join the server."));
    }
    let mut names = std::collections::HashSet::new();
    let mut pws = std::collections::HashSet::new();
    let mut open = vec![];
    for g in groups {
        let name = g.name.trim();
        if name.is_empty() {
            out.push(issue("error", "Every role needs a name."));
        } else if !names.insert(name.to_lowercase()) {
            out.push(issue("error", format!("Two roles are called “{name}” – role names must be unique.")));
        }
        if g.password.is_empty() {
            open.push(g);
        } else if !pws.insert(g.password.clone()) {
            out.push(issue("error", format!("“{name}” uses the same password as another role – passwords must be unique.")));
        }
        if g.can_edit_base && !g.can_edit_world {
            out.push(issue("error", format!("“{name}” may edit bases but not the world – the server rejects that combination.")));
        }
        if !g.password.is_empty() && g.password.chars().count() < 6 {
            out.push(issue("warn", format!("“{name}” has a very short password.")));
        }
    }
    if open.len() > 1 {
        out.push(issue("error", "Only one role may have no password."));
    }
    if let Some(o) = open.first() {
        let mine = o.permissions();
        let more = groups.iter().filter(|g| !g.password.is_empty()).any(|g| {
            let theirs = g.permissions();
            mine.iter().zip(theirs.iter()).any(|(m, t)| *m && !*t)
        });
        if more {
            out.push(issue("error", format!("“{}” has no password but more rights than a password-protected role.", o.name.trim())));
        } else {
            out.push(issue("warn", format!("“{}” has no password – anyone who finds the server joins with its permissions.", o.name.trim())));
        }
    }
    let reserved: u32 = groups.iter().map(|g| g.reserved_slots).sum();
    if reserved > c.slot_count {
        out.push(issue("error", format!("{reserved} reserved slots but only {} slots in total.", c.slot_count)));
    }
    out
}

pub fn ensure_valid(c: &ServerConfig) -> AppResult<()> {
    let errs: Vec<String> = validate(c).into_iter().filter(|i| i.level == "error").map(|i| i.message).collect();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(AppError::msg(format!("The server would reject this configuration: {}", errs.join(" "))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file a fresh 0.9 server writes (abridged).
    const REAL: &str = "{\n\t\"name\": \"Enshrouded Server\",\n\t\"saveDirectory\": \"./savegame\",\n\t\"logDirectory\": \"./logs\",\n\t\"ip\": \"0.0.0.0\",\n\t\"queryPort\": 15637,\n\t\"slotCount\": 16,\n\t\"tags\": [\n\t],\n\t\"voiceChatMode\": \"Proximity\",\n\t\"enableVoiceChat\": false,\n\t\"enableTextChat\": false,\n\t\"gameSettingsPreset\": \"Default\",\n\t\"gameSettings\": {\n\t\t\"playerHealthFactor\": 1,\n\t\t\"perkUpgradeRecyclingFactor\": 0.500000,\n\t\t\"dayTimeDuration\": 1800000000000,\n\t\t\"futureSetting\": 3\n\t},\n\t\"userGroups\": [\n\t\t{\n\t\t\t\"name\": \"Admin\",\n\t\t\t\"password\": \"AdminZT4]AnDl\",\n\t\t\t\"canKickBan\": true,\n\t\t\t\"canAccessInventories\": true,\n\t\t\t\"canEditWorld\": true,\n\t\t\t\"canEditBase\": true,\n\t\t\t\"canExtendBase\": true,\n\t\t\t\"reservedSlots\": 0\n\t\t}\n\t],\n\t\"bannedAccounts\": [\n\t\t{ \"accountIdHash\": \"abc\", \"displayName\": \"Griefer\", \"characterName\": \"Grr\", \"banDate\": 1791976428 }\n\t],\n\t\"useAlternativeP2P\": false\n}\n";

    #[test]
    fn round_trip_keeps_server_owned_keys() {
        let doc = serde_json::from_str::<Value>(REAL).unwrap().as_object().cloned().unwrap();
        let mut c = ServerConfig::default();
        read_into(&doc, &mut c);
        assert_eq!(c.user_groups.len(), 1);
        assert_eq!(c.user_groups[0].password, "AdminZT4]AnDl");
        assert_eq!(c.game_settings["dayTimeDuration"], Value::from(1_800_000_000_000i64));
        c.name = "Embervale".into();
        c.slot_count = 8;
        c.game_settings.insert("playerHealthFactor".into(), Value::from(2.0));
        c.game_settings.remove("futureSetting");
        let mut out = doc.clone();
        apply(&mut out, &c);
        let text = to_text(&out);
        assert!(text.starts_with("{\n\t\"name\": \"Embervale\""));
        assert!(text.contains("\"slotCount\": 8"));
        assert!(text.contains("\"playerHealthFactor\": 2,"), "whole floats are written as integers");
        assert!(text.contains("\"dayTimeDuration\": 1800000000000"));
        assert!(text.contains("\"futureSetting\": 3"), "settings the profile doesn't know survive");
        assert!(text.contains("Griefer"), "bans are never touched");
        assert!(text.contains("useAlternativeP2P"));
        let keys: Vec<&String> = out.keys().collect();
        assert_eq!(keys.last().unwrap().as_str(), "useAlternativeP2P", "key order is kept");
    }

    #[test]
    fn bans_parse_and_remove() {
        let dir = std::env::temp_dir().join(format!("esm-cfg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(CONFIG_FILE);
        std::fs::write(&p, REAL).unwrap();
        let bans = read_bans(&p).unwrap();
        assert_eq!(bans.len(), 1);
        assert_eq!(bans[0].display_name, "Griefer");
        assert_eq!(bans[0].ban_date, Some(1791976428));
        assert!(remove_ban(&p, &bans[0].raw).unwrap());
        assert!(read_bans(&p).unwrap().is_empty());
        assert!(history_dir(&p).read_dir().unwrap().next().is_some(), "previous version kept");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn validation_mirrors_server_rules() {
        let mut c = ServerConfig { user_groups: UserGroup::defaults(|n| format!("{n}-pw-123")), ..Default::default() };
        assert!(validate(&c).iter().all(|i| i.level != "error"));
        c.user_groups[1].password = c.user_groups[0].password.clone();
        assert!(validate(&c).iter().any(|i| i.message.contains("same password")));
        c.user_groups[1].password = "x-unique-1".into();
        c.user_groups[3].can_edit_base = true;
        assert!(validate(&c).iter().any(|i| i.message.contains("edit bases but not the world")));
        c.user_groups[3].can_edit_base = false;
        c.user_groups[3].password.clear();
        assert!(validate(&c).iter().any(|i| i.level == "warn" && i.message.contains("no password")));
        c.user_groups[2].password.clear();
        assert!(validate(&c).iter().any(|i| i.message.contains("Only one role")));
        c.user_groups[2].password = "guest-pw-9".into();
        c.user_groups[3].can_access_inventories = true; // Guest (protected) lacks it
        assert!(validate(&c).iter().any(|i| i.level == "error" && i.message.contains("more rights")));
        c.user_groups[3].can_access_inventories = false;
        c.slot_count = 2;
        c.user_groups[0].reserved_slots = 3;
        assert!(validate(&c).iter().any(|i| i.message.contains("reserved slots")));
    }
}
