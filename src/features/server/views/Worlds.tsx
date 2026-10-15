import { useCallback, useEffect, useState } from 'react';
import { ArrowRightLeft, Check, FolderOpen, FolderSearch, Globe2, History, Laptop, RefreshCw, RotateCcw, Sparkles, Trash2, Upload } from 'lucide-react';
import { api, errMsg } from '@/lib/ipc';
import { ago, bytes, dateTime, plural } from '@/lib/format';
import { pickFolder, reveal } from '@/lib/shell';
import type { LocalWorld, World } from '@/lib/types';
import { useApp, useInstance, useStatus } from '@/store/app';
import { Button } from '@/components/ui/Button';
import { confirm } from '@/components/ui/Dialog';
import { Badge, Callout, Card, Empty, PageHeader, Section } from '@/components/ui/Surface';
import { cn } from '@/lib/cn';

function SlotRow({ w, s, onRestore, disabled }: { w: World; s: World['slots'][number]; onRestore: () => void; disabled: boolean }) {
  return (
    <div className={cn('flex items-center gap-3 rounded-lg px-3 py-2', s.current ? 'bg-accent/[0.06]' : 'hover:bg-panel-2')}>
      <div className={cn('flex size-7 items-center justify-center rounded-md font-mono text-[11px]', s.current ? 'bg-accent/15 text-accent' : 'bg-panel-2 text-fg-3')}>{s.slot}</div>
      <div className="min-w-0 flex-1">
        <div className="text-[12.5px] text-fg">
          {dateTime(s.modified)} <span className="text-fg-4">· {ago(s.modified)}</span>
        </div>
        <div className="font-mono text-[10.5px] text-fg-4">
          {s.file} · {bytes(s.size)}
        </div>
      </div>
      {s.current ? (
        <Badge tone="accent">
          <Check /> Loads next
        </Badge>
      ) : (
        <Button size="sm" variant="outline" icon={<RotateCcw className="size-3.5" />} disabled={disabled} onClick={onRestore}>
          Roll back to this
        </Button>
      )}
      <span className="sr-only">{w.id}</span>
    </div>
  );
}

export function Worlds({ id }: { id: string }) {
  const inst = useInstance(id)!;
  const st = useStatus(id);
  const toast = useApp((s) => s.toast);
  const confirmOn = useApp((s) => s.settings?.confirmDestructive ?? true);
  const [worlds, setWorlds] = useState<World[] | null>(null);
  const [local, setLocal] = useState<LocalWorld[] | null>(null);
  const [picked, setPicked] = useState<{ dir: string; worlds: World[] } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const running = st?.state === 'running' || st?.state === 'starting' || st?.state === 'stopping';

  const load = useCallback(() => {
    api.listWorlds(id).then(setWorlds).catch((e) => toast('error', errMsg(e)));
    api.listLocalWorlds().then(setLocal).catch(() => setLocal([]));
  }, [id, toast]);
  useEffect(load, [load, st?.state, st?.lastSave]);

  const server = worlds?.find((w) => w.isServerWorld);
  const others = worlds?.filter((w) => !w.isServerWorld) ?? [];
  const saveDir = server?.dir ?? `${inst.installDir}\\savegame`;

  const run = async (key: string, fn: () => Promise<unknown>, ok: string) => {
    setBusy(key);
    try {
      await fn();
      toast('success', ok, id);
      load();
    } catch (e) {
      toast('error', errMsg(e), id);
    } finally {
      setBusy(null);
    }
  };

  const importWorld = async (dir: string, w: World, label: string) => {
    if (server) {
      const r = await confirm({
        title: 'Replace the server world?',
        message: `The server’s current world is backed up and moved to the quarantine folder, then “${label}” becomes the server world. Players keep their characters – those live on their own PCs.`,
        confirm: 'Replace world',
        danger: true,
      });
      if (!r.ok) return;
    }
    await run(`imp-${w.id}`, () => api.importWorld(id, dir, w.id), 'World imported – it loads on the next start');
    setPicked(null);
  };

  return (
    <div className="mx-auto max-w-[1100px] space-y-5 p-6">
      <PageHeader
        title="Worlds"
        description="A dedicated server runs one world. The game saves it about every 5 minutes and on every stop, and keeps rolling copies you can roll back to."
        actions={
          <>
            <Button size="sm" icon={<FolderOpen className="size-3.5" />} onClick={() => reveal(saveDir)}>
              Open save folder
            </Button>
            <Button size="sm" icon={<RefreshCw className="size-3.5" />} onClick={load}>
              Refresh
            </Button>
          </>
        }
      />
      {running && <Callout tone="warn">The server has the world open. Stop it to roll back, import or reset – export and backups work while it runs.</Callout>}

      <Section
        title={
          <span className="flex items-center gap-2">
            Server world {server && <span className="font-mono text-[11.5px] font-normal text-fg-4">{server.id}</span>}
          </span>
        }
        description={server ? `Last saved ${ago(server.savedAt)} · ${bytes(server.size)} · ${plural(server.slots.length, 'copy', 'copies')}` : 'No world yet – the server creates a fresh one on its first start.'}
        icon={<Globe2 />}
        actions={
          server && (
            <>
              <Button
                size="sm"
                icon={<Upload className="size-3.5" />}
                onClick={async () => {
                  const dest = await pickFolder('Export the world files to…');
                  if (dest) run('export', () => api.exportWorld(id, server.id, dest), 'World exported');
                }}
              >
                Export
              </Button>
              <Button
                size="sm"
                variant="danger"
                icon={<Sparkles className="size-3.5" />}
                disabled={running}
                loading={busy === 'reset'}
                onClick={async () => {
                  if (confirmOn) {
                    const r = await confirm({
                      title: 'Start a brand-new world?',
                      message: 'A backup is made and the current world moves to the quarantine folder (nothing is deleted). The next start generates a fresh Embervale.',
                      confirm: 'New world',
                      danger: true,
                    });
                    if (!r.ok) return;
                  }
                  run('reset', () => api.resetWorld(id), 'World reset – a new one is created on the next start');
                }}
              >
                New world
              </Button>
            </>
          )
        }
        bodyClassName="py-3"
      >
        {!worlds ? (
          <div className="py-6 text-center text-[12px] text-fg-4">Loading…</div>
        ) : !server ? (
          <Empty icon={<Globe2 />} title="No world yet" description="Start the server to generate one, or bring an existing world over from the game below." className="py-6" />
        ) : (
          <div className="space-y-1">
            <div className="mb-2 flex items-center gap-2 text-[11px] tracking-wide text-fg-4 uppercase">
              <History className="size-3.5" /> Saves, newest first
            </div>
            {server.slots.map((s) => (
              <SlotRow
                key={s.file}
                w={server}
                s={s}
                disabled={running || busy != null}
                onRestore={async () => {
                  const r = await confirm({
                    title: `Roll back to the save from ${dateTime(s.modified)}?`,
                    message: 'A backup of the current state is made first. Everything built or found after that save is gone from the world.',
                    confirm: 'Roll back',
                    danger: true,
                  });
                  if (r.ok) run(`slot-${s.slot}`, () => api.restoreWorldSlot(id, server.id, s.slot), 'Rolled back – the server loads that save next');
                }}
              />
            ))}
          </div>
        )}
      </Section>

      <Section
        title="Bring a world from the game"
        description="Move a world you started in single-player or hosted from the game client onto this server. It’s copied – your local world stays untouched."
        icon={<ArrowRightLeft />}
        actions={
          <Button
            size="sm"
            icon={<FolderSearch className="size-3.5" />}
            onClick={async () => {
              const dir = await pickFolder('Pick a folder that contains Enshrouded world files');
              if (!dir) return;
              try {
                const ws = await api.scanWorldFolder(dir);
                if (!ws.length) toast('warn', 'No Enshrouded world files in that folder (8-character names like 3ad85aea).', id);
                setPicked({ dir, worlds: ws });
              } catch (e) {
                toast('error', errMsg(e), id);
              }
            }}
          >
            From another folder…
          </Button>
        }
        bodyClassName="py-3"
      >
        {picked && (
          <div className="mb-3 space-y-1.5 rounded-xl border border-accent/30 bg-accent/[0.04] p-3">
            <div className="truncate font-mono text-[11px] text-fg-3">{picked.dir}</div>
            {picked.worlds.map((w) => (
              <div key={w.id} className="flex items-center gap-3 rounded-lg px-2 py-1.5">
                <Globe2 className="size-4 text-fg-3" />
                <span className="font-mono text-[12.5px] text-fg">{w.id}</span>
                <span className="text-[12px] text-fg-3">
                  saved {ago(w.savedAt)} · {bytes(w.size)}
                </span>
                <Button size="sm" variant="primary" className="ml-auto" disabled={running} loading={busy === `imp-${w.id}`} onClick={() => importWorld(picked.dir, w, w.id)}>
                  Use on server
                </Button>
              </div>
            ))}
          </div>
        )}
        {!local ? (
          <div className="py-5 text-center text-[12px] text-fg-4">Looking for game saves…</div>
        ) : local.length === 0 ? (
          <div className="flex items-center gap-3 py-3 text-[12px] text-fg-3">
            <Laptop className="size-4 text-fg-4" />
            No Enshrouded game saves on this PC. Looked in Saved Games\Enshrouded and Steam’s cloud folder (userdata\…\1203620\remote). Copy the world files from another PC into a folder and use “From another folder”.
          </div>
        ) : (
          <div className="divide-y divide-line">
            {local.map((l) => (
              <div key={l.world.dir + l.world.id} className="flex items-center gap-3 py-2.5">
                <Laptop className="size-4 shrink-0 text-fg-3" />
                <div className="min-w-0 flex-1">
                  <div className="text-[12.5px] text-fg">
                    World <span className="font-mono">{l.world.id}</span> <span className="text-fg-3">· saved {ago(l.world.savedAt)}</span>
                  </div>
                  <div className="truncate text-[11px] text-fg-4">
                    {l.source} · {bytes(l.world.size)} · {plural(l.world.slots.length, 'copy', 'copies')}
                  </div>
                </div>
                <Button size="sm" variant="primary" disabled={running} loading={busy === `imp-${l.world.id}`} onClick={() => importWorld(l.world.dir, l.world, l.world.id)}>
                  Use on server
                </Button>
              </div>
            ))}
          </div>
        )}
      </Section>

      {others.length > 0 && (
        <Section title="Other world files in the save folder" description="Not loaded by the dedicated server (it only uses 3ad85aea). Leftovers from copies or old setups." icon={<Globe2 />} bodyClassName="py-2">
          <div className="divide-y divide-line">
            {others.map((w) => (
              <div key={w.id} className="flex items-center gap-3 py-2.5">
                <span className="font-mono text-[12.5px] text-fg">{w.id}</span>
                <span className="text-[12px] text-fg-3">
                  saved {ago(w.savedAt)} · {bytes(w.size)}
                </span>
                <div className="ml-auto flex gap-1.5">
                  <Button size="sm" disabled={running} onClick={() => importWorld(saveDir, w, w.id)}>
                    Make server world
                  </Button>
                  <Button size="sm" variant="ghost" icon={<Trash2 className="size-3.5" />} disabled={running} onClick={() => run(`del-${w.id}`, () => api.deleteWorld(id, w.id), 'Moved to quarantine')}>
                    Remove
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </Section>
      )}

      <Card className="p-4 text-[12px] leading-relaxed text-fg-3">
        <b className="text-fg-2">Characters stay with the players.</b> Enshrouded keeps characters (level, gear, skills) on each player’s own PC, so they keep them when they join another server or you swap worlds. Only the world – terrain, bases, chests and progress – lives on the server.
      </Card>
    </div>
  );
}
