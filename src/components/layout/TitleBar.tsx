import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Command, Copy, Minus, Square, X } from 'lucide-react';
import { inTauri } from '@/lib/ipc';
import { useApp } from '@/store/app';
import { Kbd } from '@/components/ui/Surface';

/** The Flame: a spirit-teal flame over a shroud-violet ember, inside a rune ring. */
export function Logo({ className = 'size-[18px]' }: { className?: string }) {
  return (
    <svg viewBox="0 0 64 64" className={className}>
      <defs>
        <linearGradient id="esm-flame" x1="0.5" y1="1" x2="0.5" y2="0">
          <stop offset="0" stopColor="#0d8a78" />
          <stop offset="0.5" stopColor="#2dd4b7" />
          <stop offset="1" stopColor="#fff3cc" />
        </linearGradient>
        <radialGradient id="esm-ember" cx="0.5" cy="0.7" r="0.5">
          <stop offset="0" stopColor="#f3c562" />
          <stop offset="1" stopColor="#d94aa0" stopOpacity="0" />
        </radialGradient>
      </defs>
      <circle cx="32" cy="34" r="27" fill="none" stroke="#2dd4b7" strokeOpacity="0.35" strokeWidth="2.5" strokeDasharray="5 4" />
      <ellipse cx="32" cy="48" rx="14" ry="6" fill="url(#esm-ember)" />
      <path d="M32 8c3 9 13 14 13 26 0 9-6 15-13 15s-13-6-13-15c0-7 4-11 6-15 1 5 3 7 5 8-1-8 0-13 2-19z" fill="url(#esm-flame)" />
      <path d="M32 27c2 4 6 6 6 11 0 4-3 7-6 7s-6-3-6-7c0-3 2-5 3-7 1 2 2 3 3 3 0-3 0-5 0-7z" fill="#e8fffa" opacity="0.85" />
    </svg>
  );
}

export function TitleBar() {
  const [max, setMax] = useState(false);
  const setPalette = useApp((s) => s.setPalette);
  const win = inTauri() ? getCurrentWindow() : null;
  useEffect(() => {
    if (!win) return;
    win.isMaximized().then(setMax);
    const un = win.onResized(() => win.isMaximized().then(setMax));
    return () => {
      un.then((f) => f());
    };
  }, [win]);

  return (
    <div data-tauri-drag-region className="relative z-30 flex h-10 shrink-0 items-center border-b border-line bg-bg pl-3.5">
      <div data-tauri-drag-region className="flex items-center gap-2.5">
        <Logo />
        <span data-tauri-drag-region className="text-[12.5px] font-semibold tracking-tight text-fg">
          Enshrouded <span className="text-fg-3">Server Manager</span>
        </span>
      </div>
      <div data-tauri-drag-region className="flex flex-1 justify-center">
        <button
          onClick={() => setPalette(true)}
          className="flex h-7 w-[340px] items-center gap-2 rounded-lg border border-line bg-panel px-2.5 text-[12px] text-fg-4 transition-colors hover:border-line-strong hover:text-fg-3"
        >
          <Command className="size-3.5" />
          Search servers, actions, settings…
          <span className="ml-auto flex gap-0.5">
            <Kbd>Ctrl</Kbd>
            <Kbd>K</Kbd>
          </span>
        </button>
      </div>
      {win && (
        <div className="flex h-full">
          <button onClick={() => win.minimize()} className="flex h-full w-11 items-center justify-center text-fg-3 hover:bg-hover hover:text-fg">
            <Minus className="size-4" />
          </button>
          <button onClick={() => win.toggleMaximize()} className="flex h-full w-11 items-center justify-center text-fg-3 hover:bg-hover hover:text-fg">
            {max ? <Copy className="size-3.5 -scale-x-100" /> : <Square className="size-3.5" />}
          </button>
          <button onClick={() => win.close()} className="flex h-full w-11 items-center justify-center text-fg-3 hover:bg-err hover:text-white">
            <X className="size-4" />
          </button>
        </div>
      )}
    </div>
  );
}
