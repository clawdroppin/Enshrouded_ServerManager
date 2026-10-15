# Changelog

## 1.0.0 – 2026-10-14

First release, built on the Dragonwilds / Minecraft / ASA Server Manager architecture and verified against the real Enshrouded dedicated server (Steam build 23178631, game SVN 1024233).

- **Servers:** four-step wizard (server, world & players, roles, launch) with SteamCMD install; free query ports (15637, 15639 …); multi-server tabs; adopt existing installs (Steam client installs detected); duplicate, delete, `.enshrouded` packs.
- **Config:** lossless read-modify-write of `enshrouded_server.json` – bans and unknown keys are never touched; the server's own validation rules checked before every save and start; JSON editor with validation and 30 versions of history; edits while running are held until the server stops.
- **Difficulty:** preset cards plus a Custom editor for all 37 `gameSettings` (percent and minute controls, changed markers, search, reset).
- **Roles:** role cards with permission toggles, reserved slots, password generator, invite text, templates (Admin, Friend, Guest, Visitor, Builder, Moderator); passwords DPAPI-encrypted at rest.
- **Lifecycle:** graceful stop via console break (the world is saved), force stop, crash auto-restart with loop guard, re-adoption after manager restarts, hidden server console.
- **Players:** live count via Steam A2S, names and role from the log, session feed, player history with play time (SQLite), ban list with unban.
- **Worlds:** rolling autosaves with one-click rollback, export, new world, import single-player worlds from the game's save folders.
- **Backups:** interval / on stop / before update, retention, safe restore with quarantine.
- **Monitoring:** live CPU, memory, disk, network, per-core; 24 h / 7 d / 30 d history; activity timeline; live console with previous sessions.
- **Automation:** schedules, auto-update, wait for players to leave, stop when empty, memory/uptime limits, start with Windows, tray mode, Windows notifications, Discord/Slack webhooks.
- **Network & health:** port-forwarding guide with UPnP, router detection, double-NAT/CGNAT warnings, firewall fixes incl. Block rules, Steam master-list check, A2S query tool, health checks with fixes.
- **Streamer mode** hides IPs, passwords and account ids.
- **Builds:** installer (NSIS), MSI and portable editions.
- **Safety nets:** startup config errors pause auto-restart with the reason; deferred restarts never start a server you stopped; disk-space check before installs/updates; automatic Steam-listing check after start; backup integrity check; tray Start all / Stop all.
- **Faster start-up:** the JSON editor loads on demand (main bundle 1.1 MB → 0.69 MB).
