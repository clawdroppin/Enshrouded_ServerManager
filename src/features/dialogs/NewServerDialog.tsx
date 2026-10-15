import { useEffect, useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { ArrowLeft, ArrowRight, Check, Crown, Feather, Flame, FolderOpen, Globe2, Rocket, Server, Skull, Swords } from 'lucide-react';
import { cn } from '@/lib/cn';
import { api, errMsg } from '@/lib/ipc';
import { bytes, generatePassword } from '@/lib/format';
import { SERVER_COLORS } from '@/lib/colors';
import { pickFolder } from '@/lib/shell';
import { DEFAULT_GAME_SETTINGS, ROLE_TEMPLATES } from '@/data/gameplay';
import type { FolderProbe, Preset, UserGroup } from '@/lib/types';
import { useApp } from '@/store/app';
import { Button } from '@/components/ui/Button';
import { Dialog } from '@/components/ui/Dialog';
import { Checkbox, Input, Label, NumberInput, SecretInput, Select, Slider, Toggle } from '@/components/ui/Field';
import { Callout } from '@/components/ui/Surface';

const STEPS = [
  { title: 'Server', icon: Server },
  { title: 'World & players', icon: Globe2 },
  { title: 'Roles', icon: Crown },
  { title: 'Launch', icon: Rocket },
];

const PRESET_CARDS: { id: Preset; label: string; icon: typeof Flame; blurb: string }[] = [
  { id: 'Default', label: 'Default', icon: Flame, blurb: 'As designed' },
  { id: 'Relaxed', label: 'Relaxed', icon: Feather, blurb: 'Fewer enemies, more loot' },
  { id: 'Hard', label: 'Hard', icon: Swords, blurb: 'Tougher fights' },
  { id: 'Survival', label: 'Survival', icon: Skull, blurb: 'Hard + hunger' },
];

function defaultRoles(): (UserGroup & { on: boolean })[] {
  return ROLE_TEMPLATES.slice(0, 4).map((t, i) => ({
    name: t.name,
    password: generatePassword(12),
    canKickBan: t.canKickBan,
    canAccessInventories: t.canAccessInventories,
    canEditWorld: t.canEditWorld,
    canEditBase: t.canEditBase,
    canExtendBase: t.canExtendBase,
    reservedSlots: i === 0 ? 1 : 0,
    on: i < 2,
  }));
}

export function NewServerDialog() {
  const dialog = useApp((s) => s.dialog);
  const setDialog = useApp((s) => s.setDialog);
  const instances = useApp((s) => s.instances);
  const open = dialog?.kind === 'new';
  const [step, setStep] = useState(0);
  const [name, setName] = useState('');
  const [serverName, setServerName] = useState('');
  const [color, setColor] = useState('flame');
  const [dir, setDir] = useState('');
  const [port, setPort] = useState(15637);
  const [slots, setSlots] = useState(8);
  const [preset, setPreset] = useState<Preset>('Default');
  const [lookingForPlayers, setLfp] = useState(false);
  const [textChat, setTextChat] = useState(true);
  const [voiceChat, setVoiceChat] = useState(false);
  const [roles, setRoles] = useState(defaultRoles());
  const [template, setTemplate] = useState('');
  const [installNow, setInstallNow] = useState(true);
  const [probe, setProbe] = useState<FolderProbe | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const n = instances.length + 1;
    setStep(0);
    setName(n === 1 ? 'My Enshrouded Server' : `Enshrouded Server ${n}`);
    setServerName('');
    setColor(Object.keys(SERVER_COLORS)[instances.length % Object.keys(SERVER_COLORS).length]);
    setDir('');
    setTemplate(dialog?.kind === 'new' ? (dialog.templateId ?? '') : '');
    setSlots(8);
    setPreset('Default');
    setLfp(false);
    setTextChat(true);
    setVoiceChat(false);
    setRoles(defaultRoles());
    setInstallNow(true);
    setError(null);
    api.suggestPort().then(setPort).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!dir) return setProbe(null);
    const t = setTimeout(() => api.probeFolder(dir).then(setProbe).catch(() => setProbe(null)), 250);
    return () => clearTimeout(t);
  }, [dir]);

  const listName = serverName.trim() || name.trim();
  const portClash = instances.find((i) => i.config.queryPort === port);
  const chosen = roles.filter((r) => r.on);
  const reserved = chosen.reduce((n, r) => n + r.reservedSlots, 0);
  const canNext = step === 0 ? name.trim().length > 0 && !portClash : step === 1 ? listName.length > 0 : step === 2 ? chosen.length > 0 && chosen.every((r) => r.password) && reserved <= slots : true;

  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      const base = template ? instances.find((i) => i.id === template)?.config : undefined;
      const inst = await api.createInstance({
        name: name.trim(),
        installDir: dir || null,
        color,
        config: {
          name: listName,
          saveDirectory: './savegame',
          logDirectory: './logs',
          ip: '0.0.0.0',
          queryPort: port,
          slotCount: slots,
          tags: lookingForPlayers ? ['LookingForPlayers'] : [],
          voiceChatMode: base?.voiceChatMode ?? 'Proximity',
          enableVoiceChat: voiceChat,
          enableTextChat: textChat,
          gameSettingsPreset: preset,
          gameSettings: base?.gameSettings ?? DEFAULT_GAME_SETTINGS,
          userGroups: chosen.map(({ on: _on, ...g }) => g),
        },
        installNow,
        templateId: template || null,
      });
      useApp.getState().upsertInstance(inst);
      setDialog(null);
      useApp.getState().openServer(inst.id, 'overview');
      useApp.getState().toast('success', installNow ? `${inst.name} created – downloading server files…` : `${inst.name} created`, inst.id);
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={() => setDialog(null)}
      title="Create an Enshrouded server"
      description="Four quick steps. Everything can be changed later."
      icon={<Rocket />}
      width={680}
      footer={
        <>
          {step > 0 && (
            <Button variant="ghost" icon={<ArrowLeft className="size-3.5" />} onClick={() => setStep(step - 1)}>
              Back
            </Button>
          )}
          <div className="flex-1" />
          {step < STEPS.length - 1 ? (
            <Button variant="primary" iconRight={<ArrowRight className="size-3.5" />} disabled={!canNext} onClick={() => setStep(step + 1)}>
              Continue
            </Button>
          ) : (
            <Button variant="primary" icon={<Check className="size-4" />} loading={busy} onClick={create}>
              {installNow ? 'Create & install' : 'Create server'}
            </Button>
          )}
        </>
      }
    >
      <div className="mb-5 flex items-center gap-2">
        {STEPS.map((s, i) => (
          <div key={s.title} className="flex flex-1 items-center gap-2">
            <button
              disabled={i > step}
              onClick={() => setStep(i)}
              className={cn('flex items-center gap-2 rounded-lg px-2 py-1 text-[12px] font-medium whitespace-nowrap transition-colors', i === step ? 'text-fg' : i < step ? 'text-ok' : 'text-fg-4')}
            >
              <span className={cn('flex size-6 items-center justify-center rounded-full border text-[11px]', i === step ? 'border-accent bg-accent/15 text-accent' : i < step ? 'border-ok/40 bg-ok/10' : 'border-line-strong')}>
                {i < step ? <Check className="size-3" /> : i + 1}
              </span>
              {s.title}
            </button>
            {i < STEPS.length - 1 && <div className={cn('h-px flex-1', i < step ? 'bg-ok/40' : 'bg-line')} />}
          </div>
        ))}
      </div>

      <AnimatePresence mode="wait">
        <motion.div key={step} initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -8 }} transition={{ duration: 0.13 }} className="space-y-4">
          {step === 0 && (
            <>
              <div>
                <Label>Profile name</Label>
                <Input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="My Enshrouded Server" />
              </div>
              <div>
                <Label>Colour</Label>
                <div className="flex gap-2">
                  {Object.entries(SERVER_COLORS).map(([k, c]) => (
                    <button key={k} title={c.label} onClick={() => setColor(k)} className={cn('size-7 rounded-full border-2 transition-transform hover:scale-110', color === k ? 'border-fg' : 'border-transparent')} style={{ background: c.hex }} />
                  ))}
                </div>
              </div>
              <div>
                <Label>Install folder</Label>
                <div className="flex gap-2">
                  <Input mono value={dir} onChange={(e) => setDir(e.target.value)} placeholder="Automatic (inside the manager’s data folder)" />
                  <Button
                    icon={<FolderOpen className="size-3.5" />}
                    onClick={async () => {
                      const p = await pickFolder('Choose where to install the server');
                      if (p) setDir(p);
                    }}
                  >
                    Browse
                  </Button>
                </div>
                {probe && (
                  <div className={cn('mt-1.5 text-[11.5px]', !probe.writable ? 'text-err' : probe.hasServer ? 'text-ok' : 'text-fg-4')}>
                    {!probe.writable
                      ? 'This folder is not writable.'
                      : probe.hasServer
                        ? 'An Enshrouded server is already installed here – its world and roles are kept.'
                        : `${probe.empty ? 'Empty folder' : 'Folder has files'}${probe.freeBytes != null ? ` · ${bytes(probe.freeBytes)} free (needs ~9 GB)` : ''}`}
                  </div>
                )}
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Query port (UDP)</Label>
                  <NumberInput value={port} min={1024} max={65535} onChange={setPort} />
                  {portClash ? <div className="mt-1.5 text-[11.5px] text-err">Clashes with {portClash.name}.</div> : <div className="mt-1.5 text-[11.5px] text-fg-4">The only port Enshrouded needs. Default 15637.</div>}
                </div>
                {instances.length > 0 && (
                  <div>
                    <Label>Copy difficulty & automation from</Label>
                    <Select value={template} onChange={(e) => setTemplate(e.target.value)}>
                      <option value="">Defaults</option>
                      {instances.map((i) => (
                        <option key={i.id} value={i.id}>
                          {i.name}
                        </option>
                      ))}
                    </Select>
                  </div>
                )}
              </div>
            </>
          )}
          {step === 1 && (
            <>
              <div>
                <Label>Name in the Join list</Label>
                <Input value={serverName} placeholder={name} onChange={(e) => setServerName(e.target.value)} maxLength={64} />
                <div className="mt-1.5 text-[11.5px] text-fg-4">Players search for this in Play → Join.</div>
              </div>
              <div>
                <Label>Player slots · {slots}</Label>
                <Slider value={slots} min={1} max={16} onChange={setSlots} marks={[1, 4, 8, 12, 16]} />
              </div>
              <div>
                <Label>Difficulty</Label>
                <div className="grid grid-cols-4 gap-2">
                  {PRESET_CARDS.map((p) => (
                    <button
                      key={p.id}
                      onClick={() => setPreset(p.id)}
                      className={cn('flex flex-col items-start gap-1.5 rounded-xl border p-3 text-left transition-colors', preset === p.id ? 'border-accent/60 bg-accent/[0.07]' : 'border-line hover:border-line-strong')}
                    >
                      <p.icon className={cn('size-4', preset === p.id ? 'text-accent' : 'text-fg-3')} />
                      <div className="text-[12.5px] font-semibold text-fg">{p.label}</div>
                      <div className="text-[11px] text-fg-4">{p.blurb}</div>
                    </button>
                  ))}
                </div>
                <div className="mt-1.5 text-[11.5px] text-fg-4">Want to tune every value? Pick any preset now and go Custom under Difficulty afterwards.</div>
              </div>
              <div className="grid grid-cols-3 gap-3">
                {[
                  { label: 'Looking for players', v: lookingForPlayers, set: setLfp },
                  { label: 'Text chat', v: textChat, set: setTextChat },
                  { label: 'Voice chat', v: voiceChat, set: setVoiceChat },
                ].map((o) => (
                  <label key={o.label} className="flex items-center justify-between rounded-lg border border-line px-3 py-2 text-[12.5px] text-fg-2">
                    {o.label}
                    <Toggle size="sm" checked={o.v} onChange={o.set} />
                  </label>
                ))}
              </div>
            </>
          )}
          {step === 2 && (
            <>
              <div className="text-[12.5px] leading-relaxed text-fg-3">
                Players join with a role’s password and get that role’s rights. Give the Admin password only to people you trust; share Friend with your group. You can add more roles (Builder, Moderator…) later.
              </div>
              <div className="space-y-2">
                {roles.map((r, i) => (
                  <div key={r.name} className={cn('flex items-center gap-3 rounded-xl border px-3 py-2.5 transition-colors', r.on ? 'border-line-strong bg-bg-raised' : 'border-line opacity-60')}>
                    <Toggle size="sm" checked={r.on} onChange={(v) => setRoles(roles.map((x, j) => (j === i ? { ...x, on: v } : x)))} />
                    <div className="w-[150px] shrink-0">
                      <div className="flex items-center gap-1.5 text-[12.5px] font-semibold text-fg">
                        <Crown className={cn('size-3.5', r.canKickBan ? 'text-gold' : 'text-fg-4')} /> {r.name}
                      </div>
                      <div className="text-[10.5px] leading-snug text-fg-4">{ROLE_TEMPLATES[i].blurb}</div>
                    </div>
                    <div className="flex-1">
                      <SecretInput value={r.password} onChange={(v) => setRoles(roles.map((x, j) => (j === i ? { ...x, password: v } : x)))} generate />
                    </div>
                  </div>
                ))}
              </div>
              {reserved > slots && <Callout tone="err">More reserved slots than player slots.</Callout>}
            </>
          )}
          {step === 3 && (
            <>
              <div className="rounded-xl border border-line bg-bg-raised p-4">
                <dl className="grid grid-cols-[140px_1fr] gap-y-2 text-[12.5px]">
                  <dt className="text-fg-3">Profile</dt>
                  <dd className="text-fg">{name}</dd>
                  <dt className="text-fg-3">Join list name</dt>
                  <dd className="text-fg">{listName}</dd>
                  <dt className="text-fg-3">Port</dt>
                  <dd className="font-mono text-fg">UDP {port}</dd>
                  <dt className="text-fg-3">Players</dt>
                  <dd className="text-fg">
                    up to {slots} · {preset} difficulty
                  </dd>
                  <dt className="text-fg-3">Roles</dt>
                  <dd className="text-fg">{chosen.map((r) => r.name).join(', ')}</dd>
                  <dt className="text-fg-3">Folder</dt>
                  <dd className="truncate font-mono text-[11.5px] text-fg">{dir || 'automatic'}</dd>
                </dl>
              </div>
              <Checkbox checked={installNow} onChange={setInstallNow} label="Download the server files now with SteamCMD (about 4 GB download, no Steam login needed)" />
              <Callout tone="info">
                After the download, press <b className="text-fg">Start</b>. The first start creates a fresh world. To host a world you already play in single-player, bring it over under <b className="text-fg">Worlds</b> before starting.
              </Callout>
              {error && <Callout tone="err">{error}</Callout>}
            </>
          )}
        </motion.div>
      </AnimatePresence>
    </Dialog>
  );
}
