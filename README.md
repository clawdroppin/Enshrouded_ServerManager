<div align="center">

<img src="docs/icon.png" width="112" alt="Enshrouded Server Manager" />

# Enshrouded Server Manager

**Host Enshrouded dedicated servers on Windows: install, configure, monitor, back up and automate them from one fast desktop app.**

[![Latest release](https://img.shields.io/github/v/release/clawdroppin/Enshrouded_ServerManager?style=for-the-badge&color=17c3a6&label=download)](https://github.com/clawdroppin/Enshrouded_ServerManager/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/clawdroppin/Enshrouded_ServerManager/total?style=for-the-badge&color=0d8a78)](https://github.com/clawdroppin/Enshrouded_ServerManager/releases)
![Windows 10/11](https://img.shields.io/badge/Windows-10%20%7C%2011-0e1319?style=for-the-badge&logo=windows)
[![MIT](https://img.shields.io/badge/license-MIT-0e1319?style=for-the-badge)](LICENSE)

[**Download the installer**](https://github.com/clawdroppin/Enshrouded_ServerManager/releases/latest/download/Enshrouded-Server-Manager-Setup.exe) · [Portable zip](https://github.com/clawdroppin/Enshrouded_ServerManager/releases/latest/download/Enshrouded-Server-Manager-Portable.zip) · [Website](https://clawdroppin.github.io/Enshrouded_ServerManager/) · [User guide](docs/USER_GUIDE.md) · [Changelog](CHANGELOG.md)

<img src="docs/screenshots/overview.png" alt="Server overview" width="900" />

</div>

---

## Why

Running an Enshrouded server by hand means:
- SteamCMD commands
- hand-editing `enshrouded_server.json` – where one stray comma stops the server from booting
- passwords for every role, and rules the server only tells you about in its log
- nanosecond values for day length
- a firewall prompt that quietly blocks players, and updating after every patch

This app does all of that for you. It has been tested against the real dedicated server (Steam app `2278520`, build 23178631).

## Highlights

| | |
|---|---|
| **One-click servers** | A wizard picks a free query port (UDP **15637**, then 15639 …), sets up roles with strong passwords, picks a difficulty preset and downloads the server through SteamCMD (no Steam login needed). |
| **Difficulty editor** | Keen’s presets as cards, or Custom with every one of the 37 gameplay settings: sliders in % and minutes (not nanoseconds), “changed” markers, search and reset. |
| **Roles & passwords** | Admin / Friend / Guest / Visitor plus Builder and Moderator templates, permission toggles, reserved slots, password generator and one-click invite text. The server’s own validation rules are checked live, so it never refuses your config at start. |
| **Real player tracking** | Live count straight from the server (Steam A2S query), names and role from the log, join/leave feed, and a player history with play time and sessions kept in the manager’s database. |
| **Bans** | The server’s ban list with names, character names and dates – unban with a click. |
| **Worlds** | The game’s rolling autosaves with one-click rollback, export, new world, and **bring your single-player world to the server**. |
| **Backups** | Zip backups on an interval, on stop and before updates, with retention, one-click integrity check, safe restore (quarantine first) and portable `.enshrouded` server packs. |
| **Proper lifecycle** | Start, stop, restart, force stop. Stops save the world first (console break → “Saved”). Servers keep running when the manager closes and are re-adopted on launch. Crash auto-restart with a loop guard. |
| **Performance** | Live CPU, memory, disk, network and per-core charts, plus 24 h / 7 d / 30 d history of CPU, memory and players. |
| **Automation** | Schedules, auto-update, “wait for players to leave” before restarts, stop when empty, memory/uptime limits, start with Windows, tray mode (with Start all / Stop all), Windows notifications and Discord/Slack webhooks. A config error the server reports stops the restart loop instead of crashing forever. |
| **Network & health** | Port-forwarding guide with one-click UPnP, router detection, double-NAT/CGNAT warnings, firewall fixes (including the silent Block rules), an automatic “is it listed on Steam?” check after every start, an A2S query tool and health checks with fix buttons. |
| **Safe config editing** | A JSON editor for `enshrouded_server.json` with validation and 30 versions of history. Edits flow back into every page; bans and unknown keys are never touched. |
| **Privacy** | Passwords and the webhook are encrypted at rest (Windows DPAPI). Streamer mode hides IPs, passwords and account ids. |

<div align="center">
<img src="docs/screenshots/dashboard.png" width="440" alt="Dashboard" /> <img src="docs/screenshots/difficulty.png" width="440" alt="Difficulty" />
<img src="docs/screenshots/roles.png" width="440" alt="Roles" /> <img src="docs/screenshots/players.png" width="440" alt="Players" />
<img src="docs/screenshots/worlds.png" width="440" alt="Worlds" /> <img src="docs/screenshots/performance.png" width="440" alt="Performance" />
</div>

## Quick start

1. **Download** [`Enshrouded-Server-Manager-Setup.exe`](https://github.com/clawdroppin/Enshrouded_ServerManager/releases/latest/download/Enshrouded-Server-Manager-Setup.exe) and run it.
   - SmartScreen may warn because the app isn't code-signed yet: choose **More info → Run anyway**.
2. Click **New server**, give it a name, pick a difficulty and keep the generated role passwords.
3. Wait for the download (about 4 GB, 9 GB on disk), then press **Start**. It's online in a few seconds.
4. Open **Port Forwarding**: click *Forward ports automatically* (UPnP) or follow the router steps, and allow the server through Windows Firewall.
5. Friends open **Play → Join**, search your server name and enter the password of the role you gave them (Overview has copy buttons and an invite text).

The full walkthrough is in the **[User guide](docs/USER_GUIDE.md)**.

## Ports

| Port | Protocol | What it's for |
|---|---|---|
| **15637** | UDP | `queryPort` – Steam queries and player traffic |

That's the only port Enshrouded 0.9 binds; a second server uses 15639, and so on. Older guides also list 15636 – that game port no longer exists. No TCP ports are needed.

## Build from source

Requirements: Node 22+, Rust (stable, MSVC) and the WebView2 runtime (included with Windows 10/11).

```bash
npm install
npm run tauri dev      # full app with hot reload
npm run dev            # UI only, in a browser, with a mock backend
npm run build:all      # NSIS + MSI installers and a portable zip
```

Contributors (and AI assistants) should start with **[AGENTS.md](AGENTS.md)**: architecture, verified facts about the server, and the rules that keep things safe. Development happens on the `release` branch; the website lives on `gh-pages` (source in `site/`).

## FAQ

<details><summary><b>Nobody can find my server</b></summary>

Open **Network & Health** – the *Steam server list* check asks Steam whether your server is listed (that list is the in-game Join list). If it isn't, check the firewall and forward UDP 15637 under **Port Forwarding**. After a game patch, update the server: players can only join servers on their version.
</details>

<details><summary><b>How do I make someone an admin?</b></summary>

Give them the password of a role with *Kick & ban* rights (Admin by default). Rights come from the role whose password a player enters when joining. Change passwords any time under **Roles & Passwords**.
</details>

<details><summary><b>Can I move my single-player world to the server?</b></summary>

Yes – **Worlds → Bring a world from the game** lists the worlds found on this PC, or pick a folder with world files. It's copied; your own save stays untouched. Characters always stay with the players.
</details>

<details><summary><b>Do I have to keep the manager open?</b></summary>

Servers keep running without it. Crash recovery, schedules, backups, update checks and player history need the manager, so it can sit in the tray and start with Windows.
</details>

<details><summary><b>Does it support Linux servers?</b></summary>

Not yet. The manager runs and controls Windows servers only.
</details>

---

<sub>Not affiliated with or endorsed by Keen Games. Enshrouded is a trademark of Keen Games GmbH. MIT licensed.</sub>
