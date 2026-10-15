# AGENTS.md – hand-off guide for AI assistants and contributors

This is the hand-off document for **Enshrouded Server Manager**, a Windows desktop app that runs *Enshrouded* dedicated servers. Read it fully before changing code. `CLAUDE.md` points here. User-facing documentation lives in [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md); keep it in sync when behaviour changes.

## 0. Repository, branches, website

- GitHub: **https://github.com/clawdroppin/Enshrouded_ServerManager**
- **`release`** – the development line everything is pushed to (CI runs on `main` and `release`).
- **`gh-pages`** – the project website (https://clawdroppin.github.io/Enshrouded_ServerManager/). Its source is `site/index.html`; `node scripts/build-site.mjs` assembles it with `docs/screenshots` and the icon into `dist-site/`, which is what the `gh-pages` branch contains (plus `.nojekyll`).
- Releases: push a tag `vX.Y.Z` → `.github/workflows/release.yml` builds on `windows-latest` and publishes the installer, MSI and portable zip (§5).

## 1. Stack & commands

| Layer | Tech |
|---|---|
| Shell | Tauri 2: main window created in code (§4), tray icon, single-instance, autostart and notification plugins |
| Backend | Rust 2021, tokio, reqwest, sysinfo, rusqlite (bundled), zip, regex, igd-next (UPnP), windows-sys (console, DPAPI) |
| Frontend | React 19, TypeScript 7, Vite 8 (dev port **1430**), Tailwind CSS 4, Motion, Zustand, CodeMirror 6, cmdk, lucide-react |

```bash
npm install
npm run tauri dev                      # full app, hot reload
npm run dev                            # UI only in a browser with the MOCK backend (src/lib/devMock.ts)
npx vite --port 1440                   # same, on another port (the .claude "ui-preview" config)
npx tsc -p tsconfig.json --noEmit      # typecheck
cd src-tauri && cargo check            # must stay warning-free (CI uses -D warnings)
cd src-tauri && cargo test --lib       # unit tests (config, A2S, log parsing, worlds, backups, history, DPAPI)
npm run build:all                      # NSIS + MSI + portable zip (local)
SHOTS_URL=http://localhost:1440/ node scripts/screenshots.mjs   # regenerate docs/screenshots (needs a dev server)
node scripts/build-site.mjs            # assemble the GitHub Pages site into dist-site/
```

**Testing the real app.** `npx tauri build --debug --no-bundle`, put a `portable.txt` next to `src-tauri/target/debug/enshrouded-server-manager.exe` (keeps its data in `target/debug/data`), start it with `ESM_CDP_PORT=9444` and drive the WebView over the Chrome DevTools Protocol (`window.__TAURI_INTERNALS__.invoke('command', args)`). The CDP hook is compiled into debug builds only. Don't use `tauri dev` for this – its rebuild kills the process tree, including servers you started.

**Real server installs** for testing go in the git-ignored `.test-server/` folder (SteamCMD: `+force_install_dir <abs path> +login anonymous +app_update 2278520 validate +quit`; the first SteamCMD run only self-updates – run it twice).

## 2. Repository map

```
src-tauri/src/
  main.rs            entry; helper mode `--send-ctrl-c <pid>` (graceful stop, §3)
  lib.rs             setup: data dir, orphaned-WebView recovery, window, tray, plugins, command list
  commands.rs        every #[tauri::command] (thin wrappers; add new ones to lib.rs + src/lib/ipc.ts + devMock.ts)
  state.rs           AppState, per-server Runtime, TaskReporter, notify()/notify_kind() (timeline + toast + webhook), safe JSON IO
  models.rs          persisted & event types (camelCase); SERVER_APP_ID, SERVER_WORLD_ID, ServerConfig, UserGroup
  paths.rs           installed vs portable data dir, path rebasing
  core/
    config.rs        enshrouded_server.json: lossless read-modify-write, the server's own validation rules, bans
    server.rs        start / stop / restart / update orchestration
    supervisor.rs    single loop: process tracking/adoption, telemetry + per-minute history, log tail & parsing,
                     A2S polling, crash restart, schedules, wait-for-empty, idle stop, interval backups, update checks
    a2s.rs           Steam A2S_INFO / A2S_PLAYER client (player count, names, version)
    history.rs       manager.db (SQLite): metrics, event timeline, player sessions, builds
    secrets.rs       DPAPI encryption of role passwords, branch password and webhook URL at rest
    proc.rs          detached CreateProcessW launch (own hidden console), console break, priority/affinity,
                     console visibility, UDP port owners, is_within(), orphaned-WebView cleanup
    steamcmd.rs      bootstrap, app_update (beta branches), build ids (api.steamcmd.net, then SteamCMD)
    worlds.rs        world sets (3ad85aea + rolling copies + index), slot rollback, import from game saves
    backup.rs        zip backups (saves + config), restore (quarantine first), retention, .enshrouded packs
    network.rs       firewall state/rules (incl. Block rules), IPs, gateway, port suggestions, Steam listing
    upnp.rs          UPnP IGD discovery, port mapping, router info, WAN address classification
    diagnostics.rs   health checks with fix ids
src/
  store/app.ts       Zustand store: instances, statuses, tabs, tasks, toasts, dialogs; onlineCount()
  lib/live.ts        ring buffers for telemetry & console (outside React; useSyncExternalStore)
  lib/ipc.ts         typed commands + event listen (mock-aware)   lib/types.ts mirrors models.rs
  data/gameplay.ts   every gameSettings key (ranges/defaults from Keen's readme), presets, tags, role templates
  features/server/   Workspace (header, nav, shared Save bar), draft.ts, views/*
  features/dialogs/  new-server wizard, import/export/clone/delete, app settings
  features/dashboard/ components/{ui,layout,charts}/
site/                index.html – the GitHub Pages landing page (self-contained, no build step)
scripts/             build-portable.mjs, screenshots.mjs, build-site.mjs
.github/workflows/   ci.yml (check on push/PR), release.yml (tag v* -> GitHub Release)
```

## 3. Verified facts about the server

Tested against a real install (Steam build **23178631**, game SVN **1024233**, A2S version **0.0.15.0**, readme "Version 0.9.0.0", October 2026). Re-verify after big game updates.

1. **AppID 2278520** installs anonymously (~3.7 GB download, ~8.8 GB on disk). The game client is **1203620** (Steam master list `appid`, Steam Cloud folder). Layout: `enshrouded_server.exe` (console subsystem) + `.kfc`/`.dat` data files + `enshrouded_server_readme.txt` (Keen's settings reference – read it after updates).
2. **One config file:** `enshrouded_server.json` next to the exe (a `--config` switch exists, unused). First start writes it with four roles (Admin/Friend/Guest/Visitor) and **random passwords**. Bans live in **`bannedAccounts`** (the readme says `bans` – the real file says `bannedAccounts`), entries `accountIdHash`, `displayName`, `characterName`, `banDate` (unix s). Hidden key `useAlternativeP2P` exists in the binary. The server rewrites the file (adds keys, writes numbers as `1.500000`), so `config.rs` always read-modify-writes and only touches the keys the profile owns.
3. **Validation the server performs** (its `Internal Error:` strings): role names required & unique; role passwords unique; only one role without password; a passwordless role may not have more rights than a protected one; `canEditBase` requires `canEditWorld`; reserved slots ≤ slotCount; ip must parse. `config::validate` mirrors these and blocks saving/starting.
4. **`gameSettings` only apply with `gameSettingsPreset: "Custom"`** (log: `'gameSettings' will not be applied Preset is not set to …`). Durations (`fromHungerToStarving`, `dayTimeDuration`, `nightTimeDuration`) are integer **nanoseconds**. The log prints the applied settings after `[server] Game Settings 'Custom'` (floats as hex, `3fc00000` = 1.5).
5. **Ports:** the server binds **one UDP port, `queryPort` (default 15637)**, plus a random outbound socket. There is no separate game port any more (old guides: 15636). Players connect over Steam networking (relay fallback), so forwarding helps but isn't strictly required. Second server: 15639 (`suggest_port` steps by 2).
6. **Graceful stop = Ctrl+Break** to the server's console via the helper (`--send-ctrl-c`). Log: `[app] Trigger gameflow shutdown, exit: Ctrl_Break` → `[server] Saved` → exit in 1–4 s. Ctrl+C works too, but a process started from a shell that ignores Ctrl+C inherits that and swallows it – Break is always delivered. The server is launched with `CREATE_NEW_CONSOLE` + `SW_HIDE`; never share the manager's console (a debug build has one) or the break would hit the manager.
7. **Log** `logs\enshrouded_server.log`; on every start the previous file moves to `logs\backup\enshrouded_server_<ISO time>.log`. Lines: `[I 00:00:04,641] …` (level I/W/E, time since start). The tail detects a new session by the `Current UTC date:` line near the top (`log_signature`) – NTFS tunnelling can keep the creation time of a renamed-then-recreated file, so don't use timestamps. Parsed lines (unit-tested in `supervisor.rs`):
   - ready `[Session] 'HostOnline' (up)!` (≈1–5 s after launch) · version `Game Version (SVN): 1024233`
   - `[online] Server SteamId: …` · `[online] Public ipv4: …` · `[server] Saved`
   - players (format strings in the binary, **not yet seen with a real client**): `[server] Player '%k' logged in with Permissions: …`, `[server] Remove Player '%k'`, `[session] Timed out while waiting for player. Remove Player: %k`
   - fatal `Internal Error: …` / `Config Error: …`
8. **A2S works on the query port** (INFO with challenge, PLAYER; RULES gets no reply): name, players, max, version, password flag. The supervisor polls it every 10 s – the **count** is authoritative, log names fill in who. `ISteamApps/GetServersAtAddress` (no key) shows whether Steam lists the server – that list is the in-game Join list.
9. **Saves:** world container **`3ad85aea`** in `savegame\` (+ `3ad85aea-1` … rolling copies, + `3ad85aea-index` `{"latest": n, "time": <unix s>, "deleted": false}`). Autosave every ~5 min and on stop; `latest: N` means file `-N` (0 = base) – verified when `-1` appeared and the index switched to 1. Characters are stored on the players' PCs. Slot rollback copies the chosen slot over the file the index points at (never edits the index).
10. **Local game worlds** (for import) are looked for in `%USERPROFILE%\Saved Games\Enshrouded` and `Steam\userdata\<acct>\1203620\remote` – **not verified** on a PC with the game installed.
11. **Update checks:** `https://api.steamcmd.net/v1/info/2278520` → `depots.branches.<branch>.buildid` (matches the installed manifest), SteamCMD `app_info_print` as fallback.
12. **No RCON, no remote console, no remote kick.** Moderation happens in-game by a role with `canKickBan`.

## 4. Invariants & gotchas

- **The config file is shared with the server.** Never write a whole file from the profile; `config::apply` replaces managed keys only (bans and unknown keys survive). Raw edits while running go to `Runtime.pending_raw` and are written after exit. `write_text` keeps 30 versions in `_config-history`.
- **Secrets never go on the command line** and are DPAPI-sealed on disk (`secrets.rs`); a value that can't be decrypted (portable folder moved to another PC) comes back empty and is listed in `AppInfo.lostSecrets`.
- **Lifecycle operations hold `AppState::op_lock(id)`.** The supervisor uses `try_lock` and skips transitions while an operation runs. Don't add per-server loops; extend the supervisor tick.
- **Supervisor cost.** Full process discovery every 3 ticks; CPU/memory/disk only for `enshrouded_server.exe` PIDs; A2S every 10 s; one manager.db row per server per minute.
- **SteamCMD.** Wrap every call in `steam_lock`. Install folders must be absolute (`abs_dir`, which also normalises `/`).
- **Folder containment checks use `proc::is_within`**, not string prefixes.
- **Servers outlive the manager.** The supervisor adopts any `enshrouded_server.exe` under a profile's install dir and recovers version, Steam id and online players from the current log.
- **Never lose user data.** Restores, imports, slot rollbacks, world resets and deletes quarantine first and/or take a safety backup. Deleting a server moves its `Backups` out first.
- **Only the user's actions change their router or firewall** (buttons, or the per-server "Re-apply UPnP on start" opt-in).
- **Startup config errors stop the restart loop.** An `Internal Error:` / `Config Error:` logged before the server came online sets `Runtime.fatal`; the crash that follows is reported with that message and auto-restart is paused (it would fail the same way every time). The tail runs **before** the lifecycle step in each tick so a dying server's last lines are always parsed.
- **Deferred actions** (`waiting_empty`) never start a server: if it went down meanwhile, only a pending `update` still runs.
- **Disk guard:** `update_files` refuses to start SteamCMD with < 12 GB free (install) / < 4 GB (update).
- **Steam listing check:** two minutes after a server comes online the supervisor asks Steam whether it's listed and warns if not (`spawn_listing_check`).
- **Mock backend only in browser dev** (`!isTauri()`); extend `devMock.ts` whenever you add a command.
- **Charts.** Series `#0d9488` / `#d97706` and disk-usage `#0d9488` / `#d97706` / `#8b5cf6` passed the dataviz validator on the dark surface `#0e1319`. One y-axis per chart; status colours are not series colours.

## 5. Releasing

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, and add a `CHANGELOG.md` entry.
2. `cargo check` (warning-free), `cargo test --lib`, `npx tsc --noEmit` and `npm run build:all` must pass locally.
3. Commit, then `git tag vX.Y.Z && git push origin main --tags`. `release.yml` publishes `Enshrouded-Server-Manager-Setup.exe`, `.msi`, `-Portable.zip` and `SHA256SUMS.txt` (stable names for `releases/latest/download/...`). Tags with a hyphen become pre-releases. **Never move or delete a released tag.**

## 6. Ideas / not yet done

- Verify the player join/leave lines with a real client; verify the local game-save locations.
- Decode the A2S keywords (`1;1024233;1;4;1;0;1;0;32769` changes with settings) for a "settings as players see them" view.
- A2S-based "players currently online" for adopted servers whose log was rotated away.
- Per-role join statistics; Discord bot instead of a plain webhook; code signing; an auto-updater for the app; Linux (Wine) hosts.
