# CLAUDE.md

All project guidance for AI assistants lives in **[AGENTS.md](AGENTS.md)** – read it before making changes. User docs: [docs/USER_GUIDE.md](docs/USER_GUIDE.md).

Quick reminders:
- Steam AppID for the dedicated server is **2278520** (the game is 1203620).
- One UDP port: `queryPort` (default **15637**). There is no separate game port since 0.9.
- All config lives in `enshrouded_server.json` – the server writes it too (bans, new keys). Only read-modify-write the keys the profile owns (`core/config.rs`).
- Graceful stop = Ctrl+Break to the server's own hidden console → it saves and exits.
- The server world is container `3ad85aea` (+ `-N` rolling copies + `-index`).
- Keep `cargo check` warning-free, `cargo test --lib` and `npx tsc -p tsconfig.json --noEmit` passing.
- The mock backend (`src/lib/devMock.ts`) must only load in browser dev (`!isTauri()`).
