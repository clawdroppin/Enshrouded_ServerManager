import { useEffect, useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { AlertTriangle, ArrowDown, ArrowUp, Check, Copy, Crown, Plus, Send, Trash2, XCircle } from 'lucide-react';
import { cn } from '@/lib/cn';
import { api } from '@/lib/ipc';
import { copy } from '@/lib/shell';
import { generatePassword } from '@/lib/format';
import { PERMISSIONS, ROLE_TEMPLATES } from '@/data/gameplay';
import type { ConfigIssue, UserGroup } from '@/lib/types';
import { Button, IconButton } from '@/components/ui/Button';
import { Input, NumberInput, SecretInput, Toggle } from '@/components/ui/Field';
import { Menu } from '@/components/ui/Menu';
import { Badge, Callout, Card, PageHeader } from '@/components/ui/Surface';
import { useDraft } from '../draft';

type PermKey = (typeof PERMISSIONS)[number]['key'];

function RoleCard({
  g,
  idx,
  count,
  serverName,
  onChange,
  onRemove,
  onMove,
}: {
  g: UserGroup;
  idx: number;
  count: number;
  serverName: string;
  onChange: (fn: (g: UserGroup) => void) => void;
  onRemove: () => void;
  onMove: (dir: -1 | 1) => void;
}) {
  const [copied, setCopied] = useState(false);
  const perms = PERMISSIONS.filter((p) => g[p.key]).length;
  return (
    <motion.div layout initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, scale: 0.98 }} transition={{ duration: 0.16 }}>
      <Card className="overflow-hidden">
        <div className="flex items-center gap-3 border-b border-line px-4 py-3">
          <div className={cn('flex size-8 items-center justify-center rounded-lg', g.canKickBan ? 'bg-gold/15 text-gold' : 'bg-accent-dim/60 text-accent')}>
            <Crown className="size-4" />
          </div>
          <Input value={g.name} onChange={(e) => onChange((x) => void (x.name = e.target.value))} className="h-8 max-w-[220px] text-[13px] font-semibold" placeholder="Role name" />
          <Badge tone={perms === 5 ? 'accent' : perms === 0 ? 'neutral' : 'info'}>{perms === 5 ? 'Full rights' : perms === 0 ? 'Look only' : `${perms} of 5 rights`}</Badge>
          {!g.password && <Badge tone="warn">No password</Badge>}
          <div className="ml-auto flex items-center gap-1">
            <Button
              size="sm"
              variant="ghost"
              icon={copied ? <Check className="size-3.5 text-ok" /> : <Send className="size-3.5" />}
              disabled={!g.password}
              title="Copy an invite message for this role"
              onClick={() => {
                copy(`Join my Enshrouded server: search “${serverName}” under Play → Join, password: ${g.password}`);
                setCopied(true);
                setTimeout(() => setCopied(false), 1400);
              }}
            >
              Invite text
            </Button>
            <IconButton label="Move up" disabled={idx === 0} onClick={() => onMove(-1)}>
              <ArrowUp />
            </IconButton>
            <IconButton label="Move down" disabled={idx === count - 1} onClick={() => onMove(1)}>
              <ArrowDown />
            </IconButton>
            <IconButton label="Delete role" onClick={onRemove} className="hover:text-err">
              <Trash2 />
            </IconButton>
          </div>
        </div>
        <div className="grid grid-cols-[minmax(0,1fr)_240px] gap-6 px-4 py-3.5">
          <div className="space-y-2.5">
            {PERMISSIONS.map((p) => {
              const blocked = p.key === 'canEditBase' && !g.canEditWorld && g.canEditBase;
              return (
                <div key={p.key} className="flex items-center gap-3">
                  <Toggle
                    size="sm"
                    checked={g[p.key]}
                    onChange={(v) =>
                      onChange((x) => {
                        x[p.key as PermKey] = v;
                        // The server rejects "edit base but not the world" – keep them consistent.
                        if (p.key === 'canEditBase' && v) x.canEditWorld = true;
                        if (p.key === 'canEditWorld' && !v) x.canEditBase = false;
                      })
                    }
                  />
                  <div className="min-w-0">
                    <div className={cn('text-[12.5px]', g[p.key] ? 'text-fg' : 'text-fg-3', blocked && 'text-err')}>{p.label}</div>
                    <div className="text-[11px] leading-snug text-fg-4">{p.description}</div>
                  </div>
                </div>
              );
            })}
          </div>
          <div className="space-y-3">
            <div>
              <div className="mb-1.5 text-[11px] font-medium tracking-wide text-fg-4 uppercase">Password</div>
              <SecretInput value={g.password} onChange={(v) => onChange((x) => void (x.password = v))} placeholder="No password – anyone can join" generate />
            </div>
            <div>
              <div className="mb-1.5 text-[11px] font-medium tracking-wide text-fg-4 uppercase">Reserved slots</div>
              <NumberInput value={g.reservedSlots} min={0} max={16} onChange={(v) => onChange((x) => void (x.reservedSlots = v))} />
              <div className="mt-1 text-[11px] leading-snug text-fg-4">Slots kept free for this role – others see the server as full.</div>
            </div>
          </div>
        </div>
      </Card>
    </motion.div>
  );
}

export function Roles({ id }: { id: string }) {
  const { draft, patch } = useDraft(id);
  const [issues, setIssues] = useState<ConfigIssue[]>([]);
  const cfg = draft?.config;
  useEffect(() => {
    if (!cfg) return;
    const t = setTimeout(() => api.validateConfig(cfg).then(setIssues).catch(() => {}), 150);
    return () => clearTimeout(t);
  }, [cfg]);
  if (!draft) return null;
  const groups = draft.config.userGroups;
  const edit = (i: number) => (fn: (g: UserGroup) => void) => patch((d) => fn(d.config.userGroups[i]));
  const add = (t: (typeof ROLE_TEMPLATES)[number]) =>
    patch((d) => {
      let name = t.name;
      let n = 2;
      while (d.config.userGroups.some((g) => g.name.toLowerCase() === name.toLowerCase())) name = `${t.name} ${n++}`;
      d.config.userGroups.push({
        name,
        password: generatePassword(14),
        canKickBan: t.canKickBan,
        canAccessInventories: t.canAccessInventories,
        canEditWorld: t.canEditWorld,
        canEditBase: t.canEditBase,
        canExtendBase: t.canExtendBase,
        reservedSlots: 0,
      });
    });
  const roleIssues = issues.filter((i) => !/slot count|name\.|query port|IPv4|preset/i.test(i.message));
  const errors = roleIssues.filter((i) => i.level === 'error');

  return (
    <div className="mx-auto max-w-[1000px] space-y-5 p-6 pb-24">
      <PageHeader
        title="Roles & passwords"
        description="Enshrouded has no admin console: players get their rights from the role whose password they enter when joining. Give each group of people its own role and password."
        actions={
          <>
            <Button
              variant="outline"
              icon={<Copy className="size-3.5" />}
              onClick={() => copy(groups.filter((g) => g.password).map((g) => `${g.name}: ${g.password}`).join('\n'))}
              disabled={!groups.some((g) => g.password)}
            >
              Copy all passwords
            </Button>
            <Menu
              width={300}
              trigger={() => (
                <Button variant="primary" icon={<Plus className="size-4" />}>
                  Add role
                </Button>
              )}
              items={ROLE_TEMPLATES.map((t) => ({ label: t.name, hint: t.blurb.length > 26 ? '' : t.blurb, icon: <Crown />, onSelect: () => add(t) }))}
            />
          </>
        }
      />

      {errors.length > 0 && (
        <Callout tone="err" title="The server would refuse this configuration">
          <ul className="list-disc space-y-0.5 pl-4">
            {errors.map((e) => (
              <li key={e.message}>{e.message}</li>
            ))}
          </ul>
        </Callout>
      )}
      {roleIssues
        .filter((i) => i.level === 'warn')
        .map((w) => (
          <div key={w.message} className="flex items-center gap-2 rounded-lg border border-warn/25 bg-warn/6 px-3 py-2 text-[12px] text-fg-2">
            <AlertTriangle className="size-3.5 shrink-0 text-warn" /> {w.message}
          </div>
        ))}

      <div className="space-y-4">
        <AnimatePresence initial={false}>
          {groups.map((g, i) => (
            <RoleCard
              key={i}
              g={g}
              idx={i}
              count={groups.length}
              serverName={draft.config.name}
              onChange={edit(i)}
              onRemove={() => patch((d) => void d.config.userGroups.splice(i, 1))}
              onMove={(dir) =>
                patch((d) => {
                  const [x] = d.config.userGroups.splice(i, 1);
                  d.config.userGroups.splice(i + dir, 0, x);
                })
              }
            />
          ))}
        </AnimatePresence>
        {groups.length === 0 && (
          <Card className="flex flex-col items-center gap-2 py-10 text-center">
            <XCircle className="size-6 text-err" />
            <div className="text-[13px] font-semibold text-fg">No roles</div>
            <div className="text-[12px] text-fg-3">Nobody can join without at least one role. Add Admin and Friend to start.</div>
          </Card>
        )}
      </div>

      <Callout tone="info" title="Good to know">
        Role and password names must be unique, only one role may be without a password, and a role without a password can’t have more rights than a protected one – the
        server checks this on start. Bans aren’t a role: they’re kept by the server itself (Players & Bans).
      </Callout>
    </div>
  );
}
