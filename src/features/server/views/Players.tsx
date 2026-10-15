import { useCallback, useEffect, useState } from 'react';
import { Ban, Clock, Crown, History, LogIn, LogOut, RefreshCw, Search, Users } from 'lucide-react';
import { api, errMsg } from '@/lib/ipc';
import { ago, clock, dateTime, duration, mask } from '@/lib/format';
import type { BanEntry, PlayerSummary, SessionRow } from '@/lib/types';
import { onlineCount, useApp, useInstance, useStatus } from '@/store/app';
import { Button } from '@/components/ui/Button';
import { confirm } from '@/components/ui/Dialog';
import { Input, Segmented } from '@/components/ui/Field';
import { Badge, Callout, Empty, PageHeader, Section } from '@/components/ui/Surface';

function Avatar({ name }: { name: string }) {
  // A stable hue per name so the same player always looks the same.
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
  return (
    <div className="flex size-7 shrink-0 items-center justify-center rounded-full text-[12px] font-semibold text-white/90" style={{ background: `oklch(0.42 0.09 ${h})` }}>
      {name.slice(0, 1).toUpperCase()}
    </div>
  );
}

function Since({ ts }: { ts: number }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 15_000);
    return () => clearInterval(t);
  }, []);
  return <>{duration((now - ts) / 1000)}</>;
}

export function Players({ id }: { id: string }) {
  const inst = useInstance(id)!;
  const st = useStatus(id);
  const events = useApp((s) => s.players[id]) ?? [];
  const toast = useApp((s) => s.toast);
  const streamer = useApp((s) => s.settings?.streamerMode ?? false);
  const [bans, setBans] = useState<BanEntry[] | null>(null);
  const [known, setKnown] = useState<PlayerSummary[]>([]);
  const [sessions, setSessions] = useState<SessionRow[]>([]);
  const [tab, setTab] = useState<'players' | 'sessions'>('players');
  const [q, setQ] = useState('');
  const running = st?.state === 'running' || st?.state === 'starting';
  const count = onlineCount(st);

  const load = useCallback(() => {
    api.listBans(id).then(setBans).catch((e) => toast('error', errMsg(e)));
    api.knownPlayers(id).then(setKnown).catch(() => {});
    api.playerSessions(id, 300).then(setSessions).catch(() => {});
  }, [id, toast]);
  useEffect(load, [load, events.length, st?.state]);

  const unban = async (b: BanEntry) => {
    const who = b.displayName || b.characterName || b.accountIdHash;
    const r = await confirm({ title: `Unban ${who}?`, message: 'They can join again the next time the server starts.', confirm: 'Unban' });
    if (!r.ok) return;
    try {
      await api.removeBan(id, b.raw);
      toast('success', `Unbanned ${who}`, id);
      load();
    } catch (err) {
      toast('error', errMsg(err), id);
    }
  };
  const needle = q.trim().toLowerCase();
  const filteredKnown = known.filter((p) => !needle || p.name.toLowerCase().includes(needle));
  const filteredSessions = sessions.filter((s) => !needle || s.name.toLowerCase().includes(needle));
  const unnamed = running && st && st.queryPlayers != null && st.queryPlayers > st.players.length;

  return (
    <div className="mx-auto max-w-[1100px] space-y-5 p-6">
      <PageHeader
        title="Players & bans"
        description="Who's online, everyone who has played here, and the server's ban list."
        actions={
          <Button size="sm" icon={<RefreshCw className="size-3.5" />} onClick={load}>
            Refresh
          </Button>
        }
      />
      <div className="grid grid-cols-[1fr_380px] gap-4">
        <Section
          title={
            <span className="flex items-center gap-2">
              Online now <Badge tone={count ? 'ok' : 'neutral'}>{running ? `${count} / ${inst.config.slotCount}` : 'offline'}</Badge>
            </span>
          }
          description="Names from the server log; the count comes from the server itself (Steam query)."
          icon={<Users />}
          bodyClassName="py-3"
        >
          {!running ? (
            <Empty title="Server offline" description="Start the server to see who's connected." className="py-8" />
          ) : st?.players.length || unnamed ? (
            <div className="grid grid-cols-2 gap-2">
              {st!.players.map((p) => (
                <div key={p.name} className="flex items-center gap-2.5 rounded-lg border border-line bg-bg-raised px-3 py-2">
                  <Avatar name={p.name} />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[12.5px] font-medium text-fg">{p.name}</div>
                    <div className="truncate text-[10.5px] text-fg-4" title={p.permissions ?? undefined}>
                      <Clock className="mr-1 inline size-2.5" />
                      <Since ts={p.since} />
                      {p.permissions ? ` · ${p.permissions}` : ''}
                    </div>
                  </div>
                  <span className="size-2 shrink-0 rounded-full bg-ok" />
                </div>
              ))}
              {unnamed && (
                <div className="col-span-2 rounded-lg border border-dashed border-line-strong px-3 py-2 text-[12px] text-fg-3">
                  +{st!.queryPlayers! - st!.players.length} more online – they joined before the manager was watching the log.
                </div>
              )}
            </div>
          ) : (
            <Empty title="Nobody online" description="Players show up here a few seconds after they join." className="py-8" />
          )}
        </Section>
        <Section title="This session" icon={<LogIn />} bodyClassName="py-2 max-h-[260px] overflow-y-auto">
          {events.length === 0 ? (
            <div className="py-6 text-center text-[12px] text-fg-4">No joins or leaves since the manager opened.</div>
          ) : (
            [...events].reverse().map((e, i) => (
              <div key={i} className="flex items-center gap-2.5 py-1.5 text-[12.5px]">
                {e.kind === 'join' ? <LogIn className="size-3.5 text-ok" /> : <LogOut className="size-3.5 text-fg-4" />}
                <span className="text-fg">{e.name}</span>
                <span className="text-fg-3">{e.kind === 'join' ? 'joined' : 'left'}</span>
                <span className="ml-auto font-mono text-[11px] text-fg-4">{clock(e.ts)}</span>
              </div>
            ))
          )}
        </Section>
      </div>

      <Section
        title={
          <span className="flex items-center gap-2">
            Banned <Badge tone={bans?.length ? 'err' : 'neutral'}>{bans?.length ?? 0}</Badge>
          </span>
        }
        description="Bans are made in-game (Social tab) by a role with Kick & ban rights; the server stores them in enshrouded_server.json."
        icon={<Ban />}
        bodyClassName="py-2"
      >
        {running && <div className="mb-2 rounded-lg bg-warn/8 px-3 py-2 text-[12px] text-fg-2">The server keeps the ban list in memory while it runs – stop it to unban here, or unban in-game from the Social tab.</div>}
        {!bans?.length ? (
          <div className="py-5 text-center text-[12px] text-fg-4">{bans ? 'Nobody is banned.' : 'Loading…'}</div>
        ) : (
          <div className="divide-y divide-line">
            {bans.map((b) => (
              <div key={b.raw} className="flex items-center gap-3 py-2.5">
                <Avatar name={b.displayName || b.characterName || '?'} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[12.5px] font-medium text-fg">
                    {b.displayName || 'Unknown account'}
                    {b.characterName && <span className="ml-2 text-fg-3">as {b.characterName}</span>}
                  </div>
                  <div className="selectable mt-0.5 font-mono text-[11px] text-fg-4">
                    {mask(b.accountIdHash, streamer)} {b.banDate ? `· banned ${dateTime(b.banDate * (b.banDate < 1e12 ? 1000 : 1))}` : ''}
                  </div>
                </div>
                <Button size="sm" variant="outline" disabled={running} onClick={() => unban(b)}>
                  Unban
                </Button>
              </div>
            ))}
          </div>
        )}
      </Section>

      <Section
        title="History"
        description="Every player the manager has seen on this server, kept in its own database."
        icon={<History />}
        actions={
          <>
            <div className="relative w-56">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-fg-4" />
              <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Filter by name" className="h-7 pl-8" />
            </div>
            <Segmented
              size="sm"
              value={tab}
              onChange={setTab}
              options={[
                { value: 'players', label: `Players (${known.length})` },
                { value: 'sessions', label: 'Sessions' },
              ]}
            />
          </>
        }
        bodyClassName="py-2"
      >
        {tab === 'players' ? (
          filteredKnown.length === 0 ? (
            <div className="py-6 text-center text-[12px] text-fg-4">Nobody yet – players are recorded as they join.</div>
          ) : (
            <table className="w-full text-[12.5px]">
              <thead>
                <tr className="text-left text-[10.5px] tracking-wide text-fg-4 uppercase">
                  <th className="py-2 font-medium">Player</th>
                  <th className="font-medium">Role rights</th>
                  <th className="font-medium">Sessions</th>
                  <th className="font-medium">Play time</th>
                  <th className="font-medium">First seen</th>
                  <th className="text-right font-medium">Last seen</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-line">
                {filteredKnown.map((p) => {
                  const on = st?.players.some((x) => x.name === p.name);
                  return (
                    <tr key={p.name}>
                      <td className="py-2">
                        <span className="flex items-center gap-2">
                          <Avatar name={p.name} />
                          <span className="font-medium text-fg">{p.name}</span>
                          {on && <Badge tone="ok">online</Badge>}
                          {p.permissions?.toLowerCase().includes('kick') && <Crown className="size-3.5 text-gold" />}
                        </span>
                      </td>
                      <td className="max-w-[220px] truncate text-fg-3" title={p.permissions ?? undefined}>
                        {p.permissions ?? '—'}
                      </td>
                      <td className="font-mono text-fg-2">{p.sessions}</td>
                      <td className="font-mono text-fg-2">{duration(p.totalMs / 1000)}</td>
                      <td className="text-fg-3">{dateTime(p.firstSeen)}</td>
                      <td className="text-right text-fg-3">{on ? 'now' : ago(p.lastSeen)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )
        ) : filteredSessions.length === 0 ? (
          <div className="py-6 text-center text-[12px] text-fg-4">No sessions recorded yet.</div>
        ) : (
          <div className="max-h-[420px] divide-y divide-line overflow-y-auto">
            {filteredSessions.map((s) => (
              <div key={s.id} className="flex items-center gap-3 py-2 text-[12.5px]">
                <Avatar name={s.name} />
                <span className="font-medium text-fg">{s.name}</span>
                <span className="text-fg-3">{dateTime(s.joinedAt)}</span>
                <span className="ml-auto font-mono text-[11.5px] text-fg-2">{s.leftAt ? duration((s.leftAt - s.joinedAt) / 1000) : <Badge tone="ok">playing</Badge>}</span>
              </div>
            ))}
          </div>
        )}
      </Section>

      <Callout tone="info" title="No remote kick">
        Enshrouded’s dedicated server has no remote console, so kicks and bans happen in-game by a player whose role has Kick & ban rights. Player names come from the server log (the
        “logged in with Permissions” line), so the role column shows which password they used.
      </Callout>
    </div>
  );
}
