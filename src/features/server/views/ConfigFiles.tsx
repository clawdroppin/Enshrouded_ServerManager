import { useCallback, useEffect, useMemo, useState } from 'react';
import CodeMirror from '@uiw/react-codemirror';
import { StreamLanguage, HighlightStyle, indentUnit, syntaxHighlighting } from '@codemirror/language';
import { json } from '@codemirror/legacy-modes/mode/javascript';
import { EditorView } from '@codemirror/view';
import { tags as t } from '@lezer/highlight';
import { AlertTriangle, FileCode2, FolderOpen, History, RotateCcw, Save, Undo2 } from 'lucide-react';
import { cn } from '@/lib/cn';
import { api, errMsg } from '@/lib/ipc';
import { ago, dateTime } from '@/lib/format';
import { reveal } from '@/lib/shell';
import type { ConfigFile } from '@/lib/types';
import { useApp, useStatus } from '@/store/app';
import { Button, IconButton } from '@/components/ui/Button';
import { Select } from '@/components/ui/Field';
import { Callout } from '@/components/ui/Surface';

const theme = EditorView.theme(
  {
    '&': { backgroundColor: 'transparent', color: '#dde6ec' },
    '.cm-content': { caretColor: '#45e0c4', padding: '12px 0' },
    '.cm-gutters': { backgroundColor: 'transparent', color: '#4d5a65', border: 'none' },
    '.cm-activeLine': { backgroundColor: 'rgb(214 245 240 / 0.03)' },
    '.cm-activeLineGutter': { backgroundColor: 'transparent', color: '#aab8c3' },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'rgb(23 195 166 / 0.25) !important' },
    '.cm-cursor': { borderLeftColor: '#45e0c4' },
    '.cm-searchMatch': { backgroundColor: 'rgb(250 204 21 / 0.25)' },
    '.cm-panels': { backgroundColor: '#131a22', color: '#dde6ec' },
  },
  { dark: true },
);

const highlight = HighlightStyle.define([
  { tag: [t.propertyName, t.definition(t.propertyName)], color: '#7ee8d4' },
  { tag: t.string, color: '#f3c562' },
  { tag: t.number, color: '#a78bfa' },
  { tag: [t.bool, t.atom, t.keyword], color: '#f08cc4' },
  { tag: t.punctuation, color: '#74838f' },
]);

const extensions = [StreamLanguage.define(json), syntaxHighlighting(highlight), theme, indentUnit.of('\t'), EditorView.lineWrapping];

/** JSON.parse with a line number, like the server's "Failed to parse Config". */
function check(text: string): { line: number; message: string } | null {
  try {
    const v = JSON.parse(text);
    if (typeof v !== 'object' || Array.isArray(v) || v === null) return { line: 1, message: 'The file must be one JSON object { … }' };
    return null;
  } catch (e) {
    const m = String((e as Error).message);
    const pos = /position (\d+)/.exec(m)?.[1];
    const line = pos ? text.slice(0, +pos).split('\n').length : 1;
    return { line, message: m.replace(/^JSON\.parse: /, '').replace(/ in JSON at position \d+.*$/, '') };
  }
}

export function ConfigFiles({ id }: { id: string }) {
  const st = useStatus(id);
  const toast = useApp((s) => s.toast);
  const upsert = useApp((s) => s.upsertInstance);
  const refresh = useApp((s) => s.refresh);
  const [loaded, setLoaded] = useState<ConfigFile | null>(null);
  const [text, setText] = useState('');
  const [saving, setSaving] = useState(false);
  const [history, setHistory] = useState<[string, number][]>([]);
  const dirty = loaded != null && text !== loaded.text;

  const load = useCallback(async () => {
    try {
      const c = await api.readConfig(id);
      setLoaded(c);
      setText(c.text);
      api.configHistory(id).then(setHistory).catch(() => setHistory([]));
    } catch (e) {
      toast('error', errMsg(e));
    }
  }, [id, toast]);
  useEffect(() => {
    load();
  }, [load, st?.state]);

  const problem = useMemo(() => (text.trim() ? check(text) : null), [text]);

  const save = useCallback(async () => {
    setSaving(true);
    try {
      const r = await api.writeConfig(id, text);
      upsert(r.instance);
      toast('success', r.deferred ? 'Saved – written to the file when the server stops' : 'enshrouded_server.json saved', id);
      await load();
    } catch (e) {
      toast('error', errMsg(e), id);
    } finally {
      setSaving(false);
    }
  }, [id, text, load, toast, upsert]);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === 's' && dirty && !problem) {
        e.preventDefault();
        save();
      }
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, [dirty, problem, save]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex shrink-0 items-center gap-3 border-b border-line px-4 py-2.5">
        <FileCode2 className="size-4 text-accent" />
        <div className="min-w-0 flex-1">
          <div className="text-[13px] font-semibold text-fg">enshrouded_server.json</div>
          <div className="text-[11px] text-fg-4">{loaded?.modified ? `Modified ${ago(loaded.modified)}` : 'Not created yet'}</div>
        </div>
        {history.length > 0 && (
          <Select
            value=""
            className="w-48"
            onChange={async (e) => {
              if (!e.target.value) return;
              try {
                setText(await api.readTextFile(id, e.target.value));
                toast('info', 'Loaded an earlier version – review it and Save to restore');
              } catch (err) {
                toast('error', errMsg(err));
              }
            }}
          >
            <option value="">History ({history.length})</option>
            {history.map(([p, ts]) => (
              <option key={p} value={p}>
                {dateTime(ts)} · {ago(ts)}
              </option>
            ))}
          </Select>
        )}
        <IconButton label="Open folder" onClick={() => loaded && reveal(loaded.path.replace(/[\\/][^\\/]+$/, ''))}>
          <FolderOpen />
        </IconButton>
        <Button size="sm" variant="ghost" icon={<RotateCcw className="size-3.5" />} disabled={!dirty} onClick={() => setText(loaded?.text ?? '')}>
          Revert
        </Button>
        <Button size="sm" variant="primary" icon={<Save className="size-3.5" />} disabled={!dirty || !!problem} loading={saving} onClick={save}>
          Save
        </Button>
      </div>
      <div className="space-y-2 px-4 pt-3">
        {loaded?.pending && (
          <Callout
            tone="warn"
            title="Edits waiting for the server to stop"
            action={
              <Button
                size="sm"
                icon={<Undo2 className="size-3.5" />}
                onClick={async () => {
                  await api.discardPendingConfig(id);
                  await refresh();
                  load();
                }}
              >
                Discard them
              </Button>
            }
          >
            The server only reads this file when it starts and writes bans back into it, so your saved edits are held and written the moment it stops.
          </Callout>
        )}
        {!loaded?.pending && (st?.state === 'running' || st?.state === 'starting') && (
          <Callout tone="info">The server is running. A save now is held back and written when it stops – it takes effect on the next start.</Callout>
        )}
        <div className="text-[11.5px] leading-relaxed text-fg-4">
          The other pages edit this file for you. Changes made here flow back into Server Settings, Difficulty and Roles. <span className="font-mono">bannedAccounts</span> and keys
          the manager doesn’t know are left exactly as they are. The previous 30 versions are kept.
        </div>
        {loaded && !loaded.exists && <Callout tone="info">The file doesn’t exist yet – it’s written on the first start (or when you save a page).</Callout>}
      </div>
      <div className="m-4 flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-line bg-bg/70">
        <div className="min-h-0 flex-1 overflow-auto">
          <CodeMirror value={text} onChange={setText} extensions={extensions} theme="none" basicSetup={{ foldGutter: true, highlightActiveLine: true }} height="100%" style={{ height: '100%' }} />
        </div>
        <div className="flex shrink-0 items-center gap-4 border-t border-line px-4 py-1.5 font-mono text-[10.5px] text-fg-4">
          <span className="truncate">{loaded?.path}</span>
          {problem ? (
            <span className={cn('ml-auto flex items-center gap-1.5 text-err')}>
              <AlertTriangle className="size-3" /> Line {problem.line}: {problem.message}
            </span>
          ) : (
            <span className="ml-auto text-ok">Valid JSON</span>
          )}
          {history.length > 0 && (
            <span className="flex items-center gap-1">
              <History className="size-3" /> {history.length} versions kept
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
