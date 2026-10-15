# User guide

Everything you need to host an Enshrouded server with Enshrouded Server Manager – from the first install to rolling back a griefed base.

**Contents**
1. [Install the manager](#1-install-the-manager)
2. [Create your first server](#2-create-your-first-server)
3. [Let players in](#3-let-players-in)
4. [Day-to-day: start, stop, monitor](#4-day-to-day-start-stop-monitor)
5. [Server settings](#5-server-settings)
6. [Difficulty](#6-difficulty)
7. [Roles & passwords](#7-roles--passwords)
8. [Players & bans](#8-players--bans)
9. [Worlds](#9-worlds)
10. [Backups](#10-backups)
11. [Automation](#11-automation)
12. [Several servers, import, export](#12-several-servers-import-export)
13. [Troubleshooting](#13-troubleshooting)

## 1. Install the manager

- **Installer** (`Enshrouded-Server-Manager-Setup.exe`): installs for your Windows user; data lives in `%APPDATA%\com.enshroudedservermanager.app`.
- **Portable** (`…-Portable.zip`): unzip anywhere you can write to. Everything – settings, SteamCMD, servers, backups – stays in the `data` folder next to the exe, so you can move the folder or put it on another drive.

Windows SmartScreen may warn on first launch because the app isn't code-signed: **More info → Run anyway**.

A server needs about **9 GB** of disk, **8 GB of RAM** for a handful of players and **16 GB** for a full 16-player server, and a CPU with 6+ cores is comfortable.

## 2. Create your first server

Click **New server** (or press **Ctrl+N**). The wizard has four steps:

1. **Server** – a profile name, a colour, the install folder (leave empty for automatic) and the query port. A folder that already contains a server keeps its world and roles.
2. **World & players** – the name players search for in the Join list, how many slots (1–16), the difficulty preset, and whether you're *looking for players* (a browser tag) and want text/voice chat.
3. **Roles** – Admin and Friend are on by default, each with a strong generated password. Turn on Guest or Visitor if you want them.
4. **Launch** – review and download the server with SteamCMD (about 4 GB, no Steam account needed).

When the download finishes, press **Start**. The server is usually online in under 10 seconds. The Overview shows a short checklist until everything is set up.

Already have a server? **Import server → Existing folder** adopts it with its world, roles and bans. Servers installed through the Steam client (Library → Tools) are detected automatically.

## 3. Let players in

Players find your server in **Play → Join** by its name and join with a **role password**. The Overview has copy buttons for the name and every password, and Roles has an *Invite text* button per role.

Friends on your own network can join right away. For everyone else:

- **Port Forwarding** shows the one port to forward: **UDP 15637** (or whatever your query port is). *Forward ports automatically* uses UPnP; otherwise follow the router-specific steps.
- **Windows Firewall:** click *Allow through firewall* (one admin prompt). If you once dismissed Windows' "allow access?" popup, Windows created a hidden **Block** rule – the button removes it too.
- **Is it listed?** *Ask Steam if it's listed* (Port Forwarding step 5, and in Network & Health) checks Steam's server list, which is exactly what the in-game Join list shows.
- **Double NAT / CGNAT** are detected and explained. Enshrouded can still fall back to Steam's relay network in many cases, but a direct connection is better.

## 4. Day-to-day: start, stop, monitor

- **Start / Stop / Restart** live in the header of every server. **Stop** asks the server to save and shut down (it does so in a few seconds) and force-closes only after the timeout. **Force stop** in the ⋯ menu kills it without saving.
- Servers **keep running when you close the manager**. With "Keep running in the tray" (default) the manager keeps watching them; when you reopen it, it re-attaches to anything still running.
- **Overview:** join details, live CPU/memory/players/disk, recent console, version, last world save, next schedule, last backup and an activity timeline (starts, crashes, players, backups, updates).
- **Performance:** live charts (1 m – 1 h), and **24 h / 7 d / 30 d history** of players, CPU, memory and disk from the manager's database.
- **Console:** the live server log with filters, search and previous sessions (`logs\backup`).
- **Ctrl+K** opens the command palette; **Ctrl+Tab** switches tabs; **Ctrl+S** saves.

## 5. Server settings

**Server Settings** covers the listing (name, slots), **browser tags** (Looking for players, focus, languages), **chat** (text, voice, proximity or global) and the **network** (query port with a *Free port* finder, bind IP).

All settings are written into `enshrouded_server.json` for you. The server reads that file when it starts, so changes made while it runs show a **Restart to apply changes** button.

**Config File** is a JSON editor for the raw file with validation, syntax colours and the last 30 versions. Edits there flow back into the other pages. Bans and keys the manager doesn't know are always left untouched.

**Process & Launch:** show or hide the server's console window, CPU priority, which CPU cores it may use, and extra arguments.

## 6. Difficulty

Pick one of Keen's presets – **Default, Relaxed, Hard, Survival** – or **Custom**. With Custom you can tune all 37 settings, grouped into Player, Survival & death, World & time, Resources & progression, Enemies and Bosses:

- percentages for factors (player health 25–400 %, enemy damage 25–500 % …),
- minutes for day length, night length and hunger (the file stores nanoseconds – the manager converts),
- choices for weather, tombstone mode, Shroud curse, fishing, taming and enemy amounts,
- switches for durability, starvation, glider turbulence and pacified enemies.

Changed values are marked and can be reset one by one or all at once. Changing any value switches the preset to Custom, because the server ignores individual values otherwise. Settings apply from the next start, to existing worlds too.

## 7. Roles & passwords

Enshrouded has no admin console: a player's rights come from the **role** whose password they type when joining. Each role has:

- **Kick & ban** – kick and permanently ban players (Social tab in-game),
- **Chests & stations** – use containers and workstations in bases,
- **Change the world** – terraform outside bases (turn off for anti-griefing visitors),
- **Build in bases** – build and terraform inside bases (needs *Change the world*),
- **Flame Altars** – place, remove and upgrade altars,
- **Reserved slots** – slots kept free for this role.

Add roles from templates (Admin, Friend, Guest, Visitor, Builder, Moderator). The page checks the same rules the server enforces – unique names and passwords, at most one role without a password, a passwordless role can't have more rights than a protected one, reserved slots ≤ total slots – so the server never refuses to start over a role mistake.

## 8. Players & bans

- **Online now:** the count comes from the server itself, names and the role they used from the log, with how long they've been on.
- **This session:** joins and leaves since the manager opened.
- **Banned:** the server's ban list (display name, character name, date). Bans are made in-game; **Unban** works while the server is stopped (the server keeps the list in memory while running – or unban in-game).
- **History:** everyone who has played, with sessions, total play time, first and last seen – filterable.

## 9. Worlds

A dedicated server runs one world (`3ad85aea` in the save folder). The game saves about every 5 minutes and on every stop, and keeps **rolling copies**:

- **Roll back to this** makes an older save the one the server loads next (a backup is taken first, and the replaced file is kept in quarantine). Use it after griefing or a bad mistake.
- **Export** copies the world files to a folder.
- **New world** moves the current world to quarantine (nothing is deleted); the next start generates a fresh Embervale.
- **Bring a world from the game** lists worlds found on this PC (Saved Games and Steam Cloud folders) or any folder you pick, and copies one in as the server world. Your local save stays untouched.

Characters live on each player's PC, so they keep them across servers and world swaps.

## 10. Backups

Backups are zip files of the save folder plus `enshrouded_server.json`. They're made on an interval while running, after every stop and before updates (all configurable under Automation), or manually with a label. Old automatic backups are pruned past the *keep* count; manual ones are never pruned.

**Verify** reads every file in a backup and checks its checksum, so you know it can be restored before you need it. **Restore** (server stopped) quarantines the current saves first and can also restore the config file. By default backups live in `<server folder>\Backups`; App settings can move them elsewhere.

## 11. Automation

- **Uptime & recovery:** start with the manager, restart after a crash (with a delay and a crash-loop guard), stop timeout, restart above a memory limit or after an uptime, **stop when empty** after N minutes.
- **Schedules:** restart, update, backup, stop or start at a time on chosen days.
- **Wait for players to leave:** scheduled restarts/stops/updates and automatic updates wait until nobody is online (up to an hour). The header shows the waiting action with a cancel button.
- **Updates:** check every N minutes, install automatically, or update on every start.
- **Notifications:** Windows toasts (App settings) and a Discord/Slack webhook with selectable levels.
- **Safety nets:** if the server rejects its configuration at start (an `Internal Error` in the log), the manager stops auto-restarting and tells you why; installs and updates check for free disk space first; two minutes after every start the manager checks that Steam lists the server and warns if it doesn't.
- **Tray menu:** *Start all servers* and *Stop all servers (save first)* without opening the window.

Combine **Start with Windows** (App settings) with **Start with the manager** for hands-off hosting.

## 12. Several servers, import, export

- Run as many servers as your PC handles; each needs its own query port (the wizard picks a free one).
- **Duplicate** copies a server's settings (new passwords, no bans) and optionally its world.
- **Export** creates a `.enshrouded` pack (profile + config, optionally the world and passwords) to move a server to another PC or keep it as a template. **Import** installs it.
- **Delete** removes the profile, and optionally the files; backups are moved to the data folder first.

## 13. Troubleshooting

Open **Network & Health** first – it checks the files, the config file, your roles, the port, the firewall, memory, disk, the server version, the Steam listing and the last log, with a fix button where possible. **Query the server** shows exactly what the server tells Steam.

| Problem | What to do |
|---|---|
| The server stops right after starting | Console → look for `Internal Error:` or `Config Error:` lines. Network & Health explains config problems. |
| Nobody can find it | Network & Health → *Steam server list*. If missing: firewall + port forward. After a game patch: update. |
| Friends can find it but not join | Forward UDP 15637 to this PC, allow the firewall, check for CGNAT on Port Forwarding. |
| Wrong rights in-game | The player used another role's password – change the passwords under Roles. |
| "Stop the server first" | Rollbacks, imports, restores and unbans need the server stopped, because it has the files open. |
| Moved the portable folder to another PC | Encrypted passwords can't be read on another Windows account – the app tells you which ones to re-enter. |
