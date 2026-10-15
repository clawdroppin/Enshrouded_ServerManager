import { useEffect, useState } from 'react';
import { Gamepad2, Globe2, IdCard, MessageSquare, Network, Palette, Tags, Wand2 } from 'lucide-react';
import { cn } from '@/lib/cn';
import { api } from '@/lib/ipc';
import { netInfo } from '@/lib/net';
import { SERVER_COLORS } from '@/lib/colors';
import { TAGS } from '@/data/gameplay';
import type { NetInfo } from '@/lib/types';
import { useApp } from '@/store/app';
import { Button } from '@/components/ui/Button';
import { FieldRow, Input, NumberInput, Segmented, Select, Slider, TextArea, Toggle } from '@/components/ui/Field';
import { Badge, PageHeader, Section } from '@/components/ui/Surface';
import { useDraft } from '../draft';

export function Settings({ id }: { id: string }) {
  const { draft, patch } = useDraft(id);
  const instances = useApp((s) => s.instances);
  const [net, setNet] = useState<NetInfo | null>(null);
  useEffect(() => {
    netInfo().then(setNet).catch(() => {});
  }, []);
  if (!draft) return null;
  const c = draft.config;
  const clash = instances.filter((o) => o.id !== id && o.config.queryPort === c.queryPort);
  const reserved = c.userGroups.reduce((n, g) => n + g.reservedSlots, 0);
  const toggleTag = (t: string) =>
    patch((d) => {
      const has = d.config.tags.includes(t);
      d.config.tags = has ? d.config.tags.filter((x) => x !== t) : [...d.config.tags, t];
    });

  return (
    <div className="mx-auto max-w-[1000px] space-y-5 p-6 pb-24">
      <PageHeader
        title="Server Settings"
        description="The listing, chat and network keys of enshrouded_server.json. Written into the file for you – the server reads it when it starts, so changes made while it runs apply on the next restart."
      />

      <Section title="Listing" description="What players see in the in-game Join list." icon={<Globe2 />} bodyClassName="divide-y divide-line">
        <FieldRow
          label="Server name"
          description="Shown in the Join list – this is what players search for. Please keep it friendly."
          badge={<Badge tone="err">Required</Badge>}
          error={!c.name.trim() ? 'The server needs a name.' : undefined}
        >
          <Input value={c.name} invalid={!c.name.trim()} maxLength={64} onChange={(e) => patch((d) => void (d.config.name = e.target.value))} />
        </FieldRow>
        <FieldRow label="Player slots" description="slotCount – how many players can be online at once (1–16). Keen recommends 16 GB of RAM for a full server." hint={reserved > 0 ? `${reserved} of them reserved for roles` : undefined} error={reserved > c.slotCount ? 'More slots are reserved by roles than exist.' : undefined}>
          <div className="flex items-center gap-3">
            <Slider value={c.slotCount} min={1} max={16} marks={[1, 4, 8, 12, 16]} onChange={(v) => patch((d) => void (d.config.slotCount = v))} />
            <span className="w-8 text-right font-mono text-[13px] text-fg">{c.slotCount}</span>
          </div>
        </FieldRow>
      </Section>

      <Section title="Browser tags" description="Filter tags that help the right players find you (added in 0.9). Players can filter the Join list by them." icon={<Tags />} bodyClassName="space-y-3 py-4">
        {TAGS.map((g) => (
          <div key={g.group}>
            <div className="mb-1.5 text-[11px] font-medium tracking-wide text-fg-4 uppercase">{g.group}</div>
            <div className="flex flex-wrap gap-1.5">
              {g.tags.map((t) => {
                const on = c.tags.includes(t.id);
                return (
                  <button
                    key={t.id}
                    onClick={() => toggleTag(t.id)}
                    className={cn(
                      'h-7 rounded-full border px-3 text-[12px] transition-colors',
                      on ? 'border-accent/50 bg-accent/12 text-accent' : 'border-line-strong text-fg-3 hover:border-fg-4/60 hover:text-fg-2',
                    )}
                  >
                    {t.label}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </Section>

      <Section title="Chat" description="Voice and text chat for everyone on the server. Both are off on a fresh server." icon={<MessageSquare />} bodyClassName="divide-y divide-line">
        <FieldRow label="Text chat" description="enableTextChat">
          <Toggle checked={c.enableTextChat} onChange={(v) => patch((d) => void (d.config.enableTextChat = v))} />
        </FieldRow>
        <FieldRow label="Voice chat" description="enableVoiceChat">
          <Toggle checked={c.enableVoiceChat} onChange={(v) => patch((d) => void (d.config.enableVoiceChat = v))} />
        </FieldRow>
        <FieldRow label="Voice range" description="Proximity: hear players near you. Global: everyone hears everyone.">
          <Segmented
            value={c.voiceChatMode as 'Proximity' | 'Global'}
            onChange={(v) => patch((d) => void (d.config.voiceChatMode = v))}
            options={[
              { value: 'Proximity', label: 'Proximity' },
              { value: 'Global', label: 'Global' },
            ]}
          />
        </FieldRow>
      </Section>

      <Section title="Network" description="Enshrouded uses a single UDP port. Forward it on your router for players outside your network – see Port Forwarding." icon={<Network />} bodyClassName="divide-y divide-line">
        <FieldRow
          label="Query port"
          description="queryPort – Steam queries and player traffic. Default 15637; a second server on this PC gets 15639, and so on."
          error={clash.length ? `Also used by ${clash.map((x) => x.name).join(', ')} – they can't run at the same time.` : undefined}
        >
          <div className="flex gap-2">
            <NumberInput value={c.queryPort} min={1024} max={65535} onChange={(v) => patch((d) => void (d.config.queryPort = v))} className="flex-1" />
            <Button
              size="md"
              icon={<Wand2 className="size-3.5" />}
              onClick={async () => {
                const p = await api.suggestPort(id);
                patch((d) => void (d.config.queryPort = p));
              }}
            >
              Free port
            </Button>
          </div>
        </FieldRow>
        <FieldRow label="Bind to IP" description="ip – only for PCs with several network adapters. 0.0.0.0 listens on all of them (Keen: change only with a reason).">
          <Select value={c.ip} onChange={(e) => patch((d) => void (d.config.ip = e.target.value))}>
            <option value="0.0.0.0">All interfaces – 0.0.0.0 (default)</option>
            {net?.local.map((n) => (
              <option key={n.ip + n.interface} value={n.ip}>
                {n.ip} — {n.interface}
              </option>
            ))}
            {c.ip !== '0.0.0.0' && !net?.local.some((n) => n.ip === c.ip) && <option value={c.ip}>{c.ip}</option>}
          </Select>
        </FieldRow>
      </Section>

      <Section title="Manager profile" description="Only used by this app – not sent to the game." icon={<IdCard />} bodyClassName="divide-y divide-line">
        <FieldRow label="Display name" description="Name of this profile in tabs and the sidebar.">
          <Input value={draft.name} onChange={(e) => patch((d) => void (d.name = e.target.value))} />
        </FieldRow>
        <FieldRow
          label={
            <span className="flex items-center gap-2">
              <Palette className="size-3.5 text-fg-3" /> Tag colour
            </span>
          }
          description="Helps tell servers apart at a glance."
        >
          <div className="flex flex-wrap gap-2">
            {Object.entries(SERVER_COLORS).map(([k, col]) => (
              <button
                key={k}
                title={col.label}
                onClick={() => patch((d) => void (d.color = k))}
                className={cn('size-7 rounded-full border-2 transition-transform hover:scale-110', draft.color === k ? 'border-fg' : 'border-transparent')}
                style={{ background: col.hex }}
              />
            ))}
          </div>
        </FieldRow>
        <FieldRow
          label={
            <span className="flex items-center gap-2">
              <Gamepad2 className="size-3.5 text-fg-3" /> Notes
            </span>
          }
          description="Anything you want to remember about this server."
          stacked
        >
          <TextArea value={draft.notes} onChange={(e) => patch((d) => void (d.notes = e.target.value))} placeholder="Discord invite, who has which role, where the bases are…" />
        </FieldRow>
      </Section>
    </div>
  );
}
