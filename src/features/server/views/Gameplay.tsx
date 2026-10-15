import { useMemo, useState } from 'react';
import { motion } from 'motion/react';
import { Check, Feather, Flame, RotateCcw, Search, Skull, SlidersHorizontal, Swords } from 'lucide-react';
import { cn } from '@/lib/cn';
import { minToNs, nsToMin } from '@/lib/format';
import { DEFAULT_GAME_SETTINGS, GAME_SETTINGS, GROUPS, PRESETS, isDefault, settingValue, type GameSetting } from '@/data/gameplay';
import { Button } from '@/components/ui/Button';
import { Input, Segmented, Slider, Toggle } from '@/components/ui/Field';
import { Badge, Callout, PageHeader, Section } from '@/components/ui/Surface';
import { useDraft } from '../draft';

const PRESET_ICON = { Default: Flame, Relaxed: Feather, Hard: Swords, Survival: Skull, Custom: SlidersHorizontal };

function pctLabel(v: number) {
  return `${Math.round(v * 100)}%`;
}

function Control({ s, value, onChange }: { s: GameSetting; value: number | string | boolean; onChange: (v: number | string | boolean) => void }) {
  if (s.kind === 'bool') return <Toggle checked={!!value} onChange={onChange} />;
  if (s.kind === 'enum')
    return <Segmented size="sm" value={String(value)} onChange={onChange} options={s.options!.map((o) => ({ value: o.value, label: o.label }))} className="flex-wrap" />;
  if (s.kind === 'minutes') {
    const m = nsToMin(Number(value));
    return (
      <div className="flex items-center gap-3">
        <Slider value={m} min={s.min!} max={s.max!} step={s.step ?? 1} onChange={(v) => onChange(minToNs(v))} />
        <span className="w-14 text-right font-mono text-[12.5px] text-fg">{m.toFixed(m % 1 ? 1 : 0)} min</span>
      </div>
    );
  }
  const v = Number(value);
  return (
    <div className="flex items-center gap-3">
      <Slider value={v} min={s.min!} max={s.max!} step={0.05} onChange={(x) => onChange(Math.round(x * 100) / 100)} />
      <span className={cn('w-14 text-right font-mono text-[12.5px]', v > 1.001 ? 'text-gold' : v < 0.999 ? 'text-info' : 'text-fg')}>{pctLabel(v)}</span>
    </div>
  );
}

export function Gameplay({ id }: { id: string }) {
  const { draft, patch } = useDraft(id);
  const [q, setQ] = useState('');
  const gs = draft?.config.gameSettings ?? {};
  const custom = draft?.config.gameSettingsPreset === 'Custom';
  const changed = useMemo(() => GAME_SETTINGS.filter((s) => !isDefault(gs, s)).length, [gs]);
  if (!draft) return null;

  const set = (key: string, v: number | string | boolean) =>
    patch((d) => {
      d.config.gameSettings = { ...d.config.gameSettings, [key]: v };
      // Editing a value only has an effect with the Custom preset.
      d.config.gameSettingsPreset = 'Custom';
    });
  const resetAll = () => patch((d) => void (d.config.gameSettings = { ...d.config.gameSettings, ...DEFAULT_GAME_SETTINGS }));
  const needle = q.trim().toLowerCase();
  const visible = (s: GameSetting) => !needle || s.label.toLowerCase().includes(needle) || s.description.toLowerCase().includes(needle) || s.key.toLowerCase().includes(needle);

  return (
    <div className="mx-auto max-w-[1000px] space-y-5 p-6 pb-24">
      <PageHeader
        title="Difficulty & gameplay"
        description="Pick one of Keen’s presets, or go Custom and tune every value. Values are written to enshrouded_server.json and the world uses them from the next start – existing worlds pick up the change too."
      />

      <div className="grid grid-cols-5 gap-3">
        {PRESETS.map((p) => {
          const Icon = PRESET_ICON[p.id];
          const on = draft.config.gameSettingsPreset === p.id;
          return (
            <button
              key={p.id}
              onClick={() => patch((d) => void (d.config.gameSettingsPreset = p.id))}
              className={cn(
                'relative flex flex-col items-start gap-2 overflow-hidden rounded-xl border p-3.5 text-left transition-[border,background] duration-150',
                on ? 'border-accent/60 bg-accent/[0.07]' : 'border-line bg-panel hover:border-line-strong hover:bg-panel-2',
              )}
            >
              {on && <motion.span layoutId={`preset-${id}`} className="bg-flame absolute inset-x-0 top-0 h-[2px]" transition={{ duration: 0.18 }} />}
              <div className="flex w-full items-center justify-between">
                <Icon className={cn('size-5', on ? 'text-accent' : p.id === 'Survival' ? 'text-shroud' : 'text-fg-3')} />
                {on && <Check className="size-4 text-accent" />}
              </div>
              <div className="text-[13px] font-semibold text-fg">{p.label}</div>
              <div className="text-[11.5px] leading-snug text-fg-3">{p.blurb}</div>
            </button>
          );
        })}
      </div>

      {!custom && (
        <Callout tone="info" title={`The “${draft.config.gameSettingsPreset}” preset decides every value`}>
          The server ignores the individual settings below unless the preset is Custom (it logs “'gameSettings' will not be applied”). Change any value and the preset switches to Custom for you.
        </Callout>
      )}

      <div className="flex items-center gap-3">
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-fg-4" />
          <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Find a setting – e.g. night, boss, durability" className="pl-8" />
        </div>
        <Badge tone={changed ? 'accent' : 'neutral'}>{changed ? `${changed} changed from default` : 'All default'}</Badge>
        <Button size="md" variant="outline" icon={<RotateCcw className="size-3.5" />} disabled={!changed} onClick={resetAll}>
          Reset all
        </Button>
      </div>

      <div className={cn('space-y-5 transition-opacity', !custom && 'opacity-60')}>
        {GROUPS.map((g) => {
          const items = GAME_SETTINGS.filter((s) => s.group === g.id && visible(s));
          if (!items.length) return null;
          return (
            <Section key={g.id} title={g.label} description={g.blurb} bodyClassName="divide-y divide-line">
              {items.map((s) => {
                const v = settingValue(gs, s);
                const dirty = !isDefault(gs, s);
                return (
                  <div key={s.key} className="grid grid-cols-[minmax(0,1fr)_minmax(240px,360px)] items-center gap-x-8 py-3">
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 text-[13px] font-medium text-fg">
                        {s.label}
                        {dirty && (
                          <button title="Back to default" onClick={() => set(s.key, s.default)} className="flex items-center gap-1 rounded-md border border-accent/25 bg-accent/10 px-1.5 text-[10.5px] text-accent hover:bg-accent/20">
                            <RotateCcw className="size-2.5" /> changed
                          </button>
                        )}
                      </div>
                      <div className="mt-0.5 text-[12px] leading-relaxed text-fg-3">
                        {s.description}
                        <span className="ml-1.5 font-mono text-[10.5px] text-fg-4">{s.key}</span>
                      </div>
                    </div>
                    <div className={cn(s.kind === 'bool' && 'flex justify-end')}>
                      <Control s={s} value={v} onChange={(x) => set(s.key, x)} />
                    </div>
                  </div>
                );
              })}
            </Section>
          );
        })}
      </div>
    </div>
  );
}
