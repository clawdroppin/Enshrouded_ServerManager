// Fake backend for `npm run dev` in a plain browser. Never loaded inside Tauri (see main.tsx).
// Log lines and config shapes are copied from a real enshrouded_server.exe run.
import { installMock, mockEmit } from './ipc';
import { DEFAULT_GAME_SETTINGS } from '@/data/gameplay';
import type { AppSettings, BackupInfo, EventRow, MetricRow, ServerConfig, ServerInstance, ServerStatus, World } from './types';

const now = Date.now();

function makeConfig(name: string, port: number): ServerConfig {
  return {
    name,
    saveDirectory: './savegame',
    logDirectory: './logs',
    ip: '0.0.0.0',
    queryPort: port,
    slotCount: 8,
    tags: ['LookingForPlayers', 'English', 'BaseBuilding'],
    voiceChatMode: 'Proximity',
    enableVoiceChat: false,
    enableTextChat: true,
    gameSettingsPreset: 'Default',
    gameSettings: { ...DEFAULT_GAME_SETTINGS },
    userGroups: [
      { name: 'Admin', password: 'Flame7Kq2xPz', canKickBan: true, canAccessInventories: true, canEditWorld: true, canEditBase: true, canExtendBase: true, reservedSlots: 1 },
      { name: 'Friend', password: 'Ember9mWv4Ta', canKickBan: false, canAccessInventories: true, canEditWorld: true, canEditBase: true, canExtendBase: false, reservedSlots: 0 },
      { name: 'Guest', password: 'Glider3hRn8s', canKickBan: false, canAccessInventories: false, canEditWorld: true, canEditBase: false, canExtendBase: false, reservedSlots: 0 },
    ],
  };
}

function makeInstance(id: string, name: string, port: number, color: string, extra: Partial<ServerInstance> = {}): ServerInstance {
  return {
    id,
    name,
    color,
    installDir: `D:\\EnshroudedServers\\${name.replace(/\W+/g, '-')}`,
    createdAt: now - 86400000 * 3,
    notes: '',
    branch: 'public',
    branchPassword: '',
    config: makeConfig(name, port),
    launch: { upnp: false, showConsole: false, priority: 'normal', affinity: [], extraArgs: '' },
    automation: {
      autoStart: false,
      autoRestart: true,
      restartDelaySecs: 10,
      crashLimit: 3,
      stopTimeoutSecs: 60,
      schedules: [{ enabled: true, time: '05:00', days: [], action: 'restart' }],
      backupIntervalMins: 60,
      backupKeep: 24,
      backupOnStop: true,
      backupBeforeUpdate: true,
      updateOnStart: false,
      updateCheckMins: 60,
      autoUpdate: false,
      waitForEmpty: true,
      memoryLimitMb: 0,
      maxUptimeHours: 0,
      idleStopMins: 0,
    },
    ...extra,
  };
}

let instances: ServerInstance[] = [makeInstance('a1b2c3d4-0001', 'Embervale Friends', 15637, 'flame'), makeInstance('a1b2c3d4-0002', 'Survival Run', 15639, 'shroud')];
instances[1].config.gameSettingsPreset = 'Survival';

const statuses: Record<string, ServerStatus> = {};
function status(id: string, patch: Partial<ServerStatus> = {}): ServerStatus {
  const s: ServerStatus = statuses[id] ?? {
    id,
    state: 'stopped',
    pid: null,
    startedAt: null,
    readyAt: null,
    installedBuild: '23178631',
    latestBuild: '23178631',
    gameVersion: null,
    queryVersion: null,
    crashCount: 0,
    crashLoop: false,
    pendingRestart: false,
    players: [],
    queryPlayers: null,
    nextSchedule: { at: now + 3600_000 * 5, action: 'restart' },
    lastBackup: now - 1800_000,
    lastExitCode: null,
    busy: null,
    steamId: null,
    publicIp: null,
    lastSave: null,
    waiting: null,
  };
  statuses[id] = { ...s, ...patch };
  return statuses[id];
}
status('a1b2c3d4-0001', {
  state: 'running',
  pid: 18244,
  startedAt: now - 3600_000 * 2.3,
  readyAt: now - 3600_000 * 2.2,
  gameVersion: '1024233',
  queryVersion: '0.0.15.0',
  steamId: '90294873591930880',
  publicIp: '203.0.113.42',
  lastSave: now - 140_000,
  queryPlayers: 2,
  players: [
    { name: 'Flameborn_Ana', since: now - 3600_000 * 1.2, permissions: 'Can Kick Ban, Can Access Inventories, Can Edit World, Can Edit Base, Can Extend Base' },
    { name: 'ShroudWalker', since: now - 1200_000, permissions: 'Can Access Inventories, Can Edit World, Can Edit Base' },
  ],
});
status('a1b2c3d4-0002');

let settings: AppSettings = {
  steamcmdDir: null,
  defaultInstallRoot: null,
  backupDir: null,
  telemetryIntervalMs: 1000,
  confirmDestructive: true,
  reduceMotion: false,
  checkUpdatesOnLaunch: true,
  accent: 'flame',
  closeToTray: true,
  desktopNotifications: true,
  streamerMode: false,
  webhookUrl: '',
  webhookLevels: ['success', 'error'],
  metricsRetentionDays: 30,
};

const configText = (i: ServerInstance) =>
  JSON.stringify({ ...i.config, bannedAccounts: [{ accountIdHash: '8d3f…e21a', displayName: 'TorchThief', characterName: 'Grrr', banDate: Math.floor(now / 1000) - 86400 * 3 }] }, null, '\t') + '\n';

const backups: BackupInfo[] = Array.from({ length: 6 }, (_, i) => ({
  path: `D:\\EnshroudedServers\\Embervale-Friends\\Backups\\2026-10-0${9 - i}_auto.zip`,
  name: `2026-10-0${9 - i}_04-00-00_${i === 2 ? 'manual' : 'auto'}.zip`,
  size: 2_400_000 + i * 120_000,
  createdAt: now - i * 3600_000 * 7,
  label: i === 2 ? 'Before the Kindlewastes boss' : 'Interval',
  kind: i === 2 ? 'manual' : 'auto',
  files: 12,
}));

const world = (dir: string, id: string, isServer: boolean, age: number): World => ({
  id,
  isServerWorld: isServer,
  latest: 3,
  savedAt: now - age,
  deleted: false,
  size: 2_140_000,
  dir,
  slots: [3, 2, 1, 0].map((n, k) => ({ file: n === 0 ? id : `${id}-${n}`, slot: n, size: 520_000 - k * 9000, modified: now - age - k * 300_000, current: n === 3 })),
});

const LOG_SAMPLES = [
  "[I 00:00:04,641] [Session] 'HostOnline' (up)!",
  '[I 00:05:06,424] [savexxx] SAVE 4 bases 1,932 entities size 512,330',
  '[I 00:05:06,455] [server] Saved',
  '[I 00:31:01,048] -------------- Session ----------------',
  '[I 00:31:01,048]   m#0(128): up 0 (0), down 0 (0), remote 0 (0), limit 256, lost 0, ping 0 ms',
  '[W 00:33:12,200] [online] Peer #2 high latency 210 ms',
  '[I 00:35:11,011] [Server][Water] Added Water Dispenser: id: [ 3380609024 ]',
];

function emitTelemetry() {
  const t = Date.now() / 1000;
  const running = instances.filter((i) => statuses[i.id]?.state === 'running' || statuses[i.id]?.state === 'starting');
  mockEmit('telemetry', {
    ts: Date.now(),
    system: {
      cpu: 22 + Math.sin(t / 7) * 8 + Math.random() * 6,
      memUsed: 24.2e9 + Math.sin(t / 30) * 4e8,
      memTotal: 68.4e9,
      rxBps: 180_000 + Math.random() * 90_000,
      txBps: 260_000 + Math.random() * 120_000,
      cores: 24,
      perCore: Array.from({ length: 24 }, (_, i) => Math.max(1, 25 + Math.sin(t / 3 + i) * 20 + Math.random() * 12)),
    },
    servers: running.map((i, k) => ({
      id: i.id,
      cpu: 4 + k * 3 + Math.sin(t / 5) * 2.5 + Math.random() * 2,
      mem: 2.4e9 + k * 6e8 + Math.sin(t / 40) * 1.2e8,
      virt: 4.6e9,
      readBps: Math.random() < 0.2 ? Math.random() * 2e6 : Math.random() * 40_000,
      writeBps: Math.random() < 0.1 ? 4e6 + Math.random() * 3e6 : Math.random() * 120_000,
      uptime: Math.round((Date.now() - (statuses[i.id].startedAt ?? Date.now())) / 1000),
    })),
  });
}

function emitStatus(id: string, patch: Partial<ServerStatus>) {
  mockEmit('server-status', status(id, patch));
}

function log(id: string, source: string, line: string) {
  mockEmit('log-lines', { id, source, lines: [line], ts: Date.now() });
}

const events: EventRow[] = [
  { id: 6, serverId: 'a1b2c3d4-0001', ts: now - 1200_000, level: 'info', kind: 'player', message: 'ShroudWalker joined Embervale Friends' },
  { id: 5, serverId: 'a1b2c3d4-0001', ts: now - 3600_000 * 1.2, level: 'info', kind: 'player', message: 'Flameborn_Ana joined Embervale Friends' },
  { id: 4, serverId: 'a1b2c3d4-0001', ts: now - 3600_000 * 2.2, level: 'success', kind: 'lifecycle', message: 'Embervale Friends is online' },
  { id: 3, serverId: 'a1b2c3d4-0001', ts: now - 3600_000 * 2.3, level: 'success', kind: 'update', message: 'Updated to build 23178631' },
  { id: 2, serverId: 'a1b2c3d4-0001', ts: now - 3600_000 * 26, level: 'error', kind: 'crash', message: 'Embervale Friends stopped unexpectedly – restarting in 10s' },
  { id: 1, serverId: 'a1b2c3d4-0001', ts: now - 3600_000 * 30, level: 'info', kind: 'config', message: 'Server settings changed' },
];

function metrics(since: number): MetricRow[] {
  const out: MetricRow[] = [];
  for (let ts = Math.max(since, now - 7 * 86400_000); ts < now; ts += 60_000 * 5) {
    const h = new Date(ts).getHours();
    const evening = h >= 18 && h <= 23 ? 1 : h >= 12 ? 0.5 : 0.1;
    const players = Math.max(0, Math.round(evening * 4 + Math.sin(ts / 3.6e6) * 1.5));
    out.push({ ts, cpu: 3 + players * 1.8 + Math.random(), mem: 2.2e9 + players * 1.4e8, readBps: Math.random() * 6e4, writeBps: Math.random() * 1.6e5, players });
  }
  return out;
}

export function installDevMock() {
  const find = (id: unknown) => instances.find((i) => i.id === id)!;
  installMock({
    get_app_info: () => ({ version: '1.0.0', dataDir: 'C:\\Users\\me\\AppData\\Roaming\\com.enshroudedservermanager.app', portable: false, appId: 2278520, cores: 24, totalMemory: 68.4e9, os: 'Windows 11 Pro', hostname: 'EMBERVALE-PC', lostSecrets: [] }),
    get_settings: () => settings,
    save_settings: ({ settings: s }) => void (settings = s as AppSettings),
    cancel_task: () => undefined,
    test_webhook: () => undefined,
    app_quit: () => undefined,
    window_hide: () => undefined,
    probe_folder: () => ({ exists: true, writable: true, empty: true, hasServer: false, freeBytes: 812e9 }),
    list_instances: () => instances,
    get_statuses: () => instances.map((i) => status(i.id)),
    suggest_port: () => 15637 + instances.length * 2,
    create_instance: ({ input }) => {
      const inp = input as { name: string; color: string; config: ServerConfig; installNow: boolean };
      const inst = makeInstance(`mock-${Date.now()}`, inp.name, inp.config.queryPort, inp.color);
      inst.config = inp.config;
      instances = [...instances, inst];
      status(inst.id, { state: inp.installNow ? 'installing' : 'notInstalled', installedBuild: null });
      if (inp.installNow) {
        const task = `t-${Date.now()}`;
        let p = 0;
        const iv = setInterval(() => {
          p += 0.08;
          mockEmit('task', { taskId: task, instanceId: inst.id, kind: 'steamcmd', title: `Installing ${inst.name}`, message: `Downloading – ${(p * 100).toFixed(1)}% (${(p * 3.73).toFixed(2)} / 3.73 GB)`, progress: Math.min(1, p), done: p >= 1, error: null });
          if (p >= 1) {
            clearInterval(iv);
            emitStatus(inst.id, { state: 'stopped', installedBuild: '23178631' });
          }
        }, 400);
      }
      return inst;
    },
    update_instance: ({ instance }) => {
      const i = instance as ServerInstance;
      instances = instances.map((x) => (x.id === i.id ? i : x));
      if (statuses[i.id]?.state === 'running') emitStatus(i.id, { pendingRestart: true });
      return i;
    },
    validate_config: ({ config }) => {
      const c = config as ServerConfig;
      const out: { level: string; message: string }[] = [];
      const pws = c.userGroups.map((g) => g.password).filter(Boolean);
      if (new Set(pws).size !== pws.length) out.push({ level: 'error', message: 'Two roles share a password – passwords must be unique.' });
      for (const g of c.userGroups) {
        if (g.canEditBase && !g.canEditWorld) out.push({ level: 'error', message: `“${g.name}” may edit bases but not the world – the server rejects that combination.` });
        if (!g.password) out.push({ level: 'warn', message: `“${g.name}” has no password – anyone who finds the server joins with its permissions.` });
      }
      return out;
    },
    reorder_instances: () => undefined,
    delete_instance: ({ id }) => void (instances = instances.filter((i) => i.id !== id)),
    clone_instance: ({ id, name }) => {
      const src = find(id);
      const c = { ...structuredClone(src), id: `mock-${Date.now()}`, name: name as string, config: { ...structuredClone(src.config), queryPort: 15637 + instances.length * 2 } };
      instances = [...instances, c];
      status(c.id, { state: 'notInstalled', installedBuild: null });
      return c;
    },
    find_existing_installs: () => ['G:\\SteamLibrary\\steamapps\\common\\EnshroudedServer'],
    import_existing: ({ path }) => {
      const c = makeInstance(`mock-${Date.now()}`, 'Imported Server', 15645, 'sky', { installDir: path as string });
      instances = [...instances, c];
      status(c.id);
      return c;
    },
    install_server: ({ id }) => {
      emitStatus(id as string, { state: 'updating' });
      setTimeout(() => emitStatus(id as string, { state: 'stopped' }), 2500);
    },
    check_update: () => false,
    start_server: ({ id }) => {
      const sid = id as string;
      emitStatus(sid, { state: 'starting', pid: 4242, startedAt: Date.now(), players: [] });
      log(sid, 'manager', 'Launching enshrouded_server.exe (PID 4242)');
      setTimeout(() => {
        log(sid, 'server', "[I 00:00:04,641] [Session] 'HostOnline' (up)!");
        emitStatus(sid, { state: 'running', readyAt: Date.now(), queryPlayers: 0, gameVersion: '1024233', queryVersion: '0.0.15.0', publicIp: '203.0.113.42' });
        mockEmit('server-event', { id: sid, level: 'success', message: `${find(sid).name} is online`, ts: Date.now() });
      }, 2500);
    },
    stop_server: ({ id }) => {
      const sid = id as string;
      emitStatus(sid, { state: 'stopping' });
      log(sid, 'manager', 'Asking the server to save and shut down…');
      setTimeout(() => {
        log(sid, 'server', '[I 00:41:22,320] [app] Trigger gameflow shutdown, exit: Ctrl_Break');
        log(sid, 'server', '[I 00:41:22,528] [server] Saved');
        log(sid, 'manager', 'Server saved and stopped.');
        emitStatus(sid, { state: 'stopped', pid: null, startedAt: null, players: [], queryPlayers: null, pendingRestart: false });
      }, 1800);
    },
    restart_server: ({ id }) => {
      const sid = id as string;
      emitStatus(sid, { state: 'restarting' });
      setTimeout(() => emitStatus(sid, { state: 'running', startedAt: Date.now(), pendingRestart: false }), 3000);
    },
    cancel_deferred: ({ id }) => emitStatus(id as string, { waiting: null }),
    get_launch_line: ({ id }) => `"${find(id).installDir}\\enshrouded_server.exe"`,
    set_console_visible: () => 1,
    clear_crash_state: () => undefined,
    query_server: ({ id }) => ({
      info: statuses[id as string]?.state === 'running' ? { name: find(id).config.name, players: statuses[id as string].queryPlayers ?? 0, maxPlayers: find(id).config.slotCount, version: '0.0.15.0', password: true, keywords: '1;1024233;1;0;1;0;0;0;0', pingMs: 1 } : null,
      players: (statuses[id as string]?.players ?? []).map((p) => ({ name: p.name, score: 0, durationSecs: (Date.now() - p.since) / 1000 })),
    }),
    read_config: ({ id }) => ({ path: `${find(id).installDir}\\enshrouded_server.json`, exists: true, text: configText(find(id)), modified: now - 60000, pending: false }),
    write_config: ({ id, text }) => {
      const i = find(id);
      const v = JSON.parse(text as string);
      i.config = { ...i.config, ...v };
      return { deferred: statuses[id as string]?.state === 'running', instance: i };
    },
    discard_pending_config: () => undefined,
    list_config_history: () => [['D:\\x\\_config-history\\enshrouded_server.20261009-101500.json', now - 3600_000]],
    read_text_file: ({ id }) => configText(find(id)),
    list_bans: () => [{ raw: '{"displayName":"TorchThief"}', accountIdHash: '8d3f2c9b7e21a', displayName: 'TorchThief', characterName: 'Grrr', banDate: Math.floor(now / 1000) - 86400 * 3 }],
    remove_ban: () => true,
    read_log_tail: () => [
      '[I 00:00:00,081] enshrouded_server(detached HEAD) - version b466cef1500d760b8ba3dda230001923c40d0e12 (master)',
      "[I 00:00:00,081] Config Parsed: 'D:/EnshroudedServers/Embervale-Friends/enshrouded_server.json'",
      '[I 00:00:01,007] Game Version (SVN): 1024233',
      "[I 00:00:01,099] [server] Game Settings 'Default'",
      '[I 00:00:04,499] [online] Server connected to Steam successfully',
      '[I 00:00:04,626] [online] Server SteamId: 90294873591930880',
      ...LOG_SAMPLES,
      "[I 01:02:03,000] [server] Player 'Flameborn_Ana' logged in with Permissions: Can Kick Ban, Can Access Inventories, Can Edit World, Can Edit Base, Can Extend Base",
    ],
    list_log_files: () => [{ path: 'D:\\x\\logs\\backup\\enshrouded_server_2026-10-14T07-14-49.log', name: 'enshrouded_server_2026-10-14T07-14-49.log', size: 63_168, modified: now - 86400_000 }],
    list_worlds: ({ id }) => (find(id).id === 'a1b2c3d4-0001' ? [world(`${find(id).installDir}\\savegame`, '3ad85aea', true, 140_000)] : []),
    list_local_worlds: () => [{ world: world('C:\\Users\\me\\Saved Games\\Enshrouded', '5f3c9a1b', false, 86400_000 * 2), source: 'Saved Games' }],
    scan_world_folder: ({ path }) => [world(path as string, '7a0be311', false, 86400_000 * 5)],
    import_world: () => 4,
    restore_world_slot: () => undefined,
    export_world: () => 4,
    delete_world: () => undefined,
    reset_world: () => null,
    metrics_history: ({ since }) => metrics(since as number),
    event_history: ({ id }) => events.filter((e) => !id || e.serverId === id),
    player_sessions: () => [
      { id: 3, name: 'ShroudWalker', joinedAt: now - 1200_000, leftAt: null, permissions: 'Can Access Inventories, Can Edit World, Can Edit Base' },
      { id: 2, name: 'Flameborn_Ana', joinedAt: now - 3600_000 * 1.2, leftAt: null, permissions: 'Can Kick Ban, Can Access Inventories' },
      { id: 1, name: 'ShroudWalker', joinedAt: now - 86400_000, leftAt: now - 86400_000 + 5400_000, permissions: 'Can Access Inventories' },
    ],
    known_players: () => [
      { name: 'ShroudWalker', sessions: 7, totalMs: 3600_000 * 11.5, firstSeen: now - 86400_000 * 6, lastSeen: now, permissions: 'Can Access Inventories, Can Edit World, Can Edit Base' },
      { name: 'Flameborn_Ana', sessions: 12, totalMs: 3600_000 * 26, firstSeen: now - 86400_000 * 9, lastSeen: now, permissions: 'Can Kick Ban, Can Access Inventories, Can Edit World, Can Edit Base, Can Extend Base' },
      { name: 'HollowSeeker', sessions: 2, totalMs: 3600_000 * 1.7, firstSeen: now - 86400_000 * 4, lastSeen: now - 86400_000 * 3, permissions: 'Can Edit World' },
    ],
    build_history: () => [{ buildId: '23178631', branch: 'public', ts: now - 3600_000 * 2.3 }],
    create_backup: ({ label }) => {
      const b = { ...backups[0], name: 'new.zip', path: `D:\\x\\${Date.now()}.zip`, createdAt: Date.now(), label: (label as string) || 'Manual backup', kind: 'manual' };
      backups.unshift(b);
      return b;
    },
    list_backups: () => backups,
    get_backup_dir: () => 'D:\\EnshroudedServers\\Embervale-Friends\\Backups',
    restore_backup: () => 12,
    verify_backup: () => ({ ok: true, files: 12, worldFiles: 11, hasConfig: true, bytes: 2_310_000, error: null }),
    delete_backup: ({ path }) => {
      const i = backups.findIndex((b) => b.path === path);
      if (i >= 0) backups.splice(i, 1);
    },
    export_pack: () => undefined,
    read_pack: () => ({ format: 1, appVersion: '1.0.0', exportedAt: now - 86400_000, instance: makeInstance('x', 'Friend’s Server', 15637, 'gold'), hasSaves: true, hasConfig: true, includesSecrets: false }),
    import_pack: ({ name }) => {
      const c = makeInstance(`mock-${Date.now()}`, (name as string) || 'Imported', 15647, 'gold');
      instances = [...instances, c];
      status(c.id, { state: 'notInstalled', installedBuild: null });
      return c;
    },
    firewall_state: () => ({ ruleExists: true, portsOk: true, rulePorts: '15637', programRule: true, blockedBy: [] }),
    add_firewall_rules: () => undefined,
    remove_firewall_rules: () => undefined,
    net_info: () => ({ publicIp: '203.0.113.42', local: [{ interface: 'Ethernet', ip: '192.168.1.20' }], gateway: '192.168.1.1' }),
    steam_listing: () => [{ addr: '203.0.113.42:15637', gameport: 15637, appid: 1203620, steamid: '90294873923053568' }],
    upnp_probe: () => ({
      found: true,
      gateway: '192.168.1.1',
      routerName: 'Archer AX55',
      manufacturer: 'TP-Link',
      model: 'AX3000',
      externalIp: '203.0.113.42',
      wanKind: 'public',
      mappings: [{ externalPort: 15637, internalPort: 15637, internalClient: '192.168.1.20', description: 'Enshrouded Embervale Friends (15637)', enabled: true }],
      error: null,
    }),
    upnp_map: ({ id }) => [`UDP ${find(id).config.queryPort} → 192.168.1.20:${find(id).config.queryPort} forwarded`],
    upnp_unmap: () => undefined,
    run_diagnostics: () => [
      { id: 'install', title: 'Server files', level: 'ok', detail: 'D:\\EnshroudedServers\\Embervale-Friends\\enshrouded_server.exe', fix: '' },
      { id: 'configfile', title: 'enshrouded_server.json', level: 'ok', detail: 'The file parses as valid JSON.', fix: '' },
      { id: 'roles', title: 'Roles & settings', level: 'ok', detail: '3 roles, every one password protected.', fix: '' },
      { id: 'ports', title: 'Query port', level: 'ok', detail: 'UDP 15637 is available (used by this server).', fix: '' },
      { id: 'firewall', title: 'Windows Firewall', level: 'warn', detail: 'No inbound rule for this server – players outside your PC may not reach it.', fix: 'firewall' },
      { id: 'ram', title: 'Memory', level: 'ok', detail: '63.8 GB installed – 8 GB recommended for 8 slots.', fix: '' },
      { id: 'disk', title: 'Disk space', level: 'ok', detail: '812.0 GB free on D:\\.', fix: '' },
      { id: 'listing', title: 'Steam server list', level: 'ok', detail: 'Listed as 203.0.113.42:15637 – players can find “Embervale Friends” in the Join menu.', fix: '' },
    ],
    disk_usage: () => ({ install: 8.9e9, saves: 2.1e6, backups: 18e6, logs: 1.2e6 }),
  });

  setInterval(emitTelemetry, 1000);
  let k = 0;
  setInterval(() => {
    for (const i of instances) if (statuses[i.id]?.state === 'running') log(i.id, 'server', LOG_SAMPLES[k++ % LOG_SAMPLES.length]);
  }, 1700);
}
