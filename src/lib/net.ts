// Shared network helpers and a short-lived cache for slow lookups.
import { api } from './ipc';
import type { NetInfo, ServerInstance } from './types';

export interface PortRow {
  port: number;
  label: string;
  purpose: string;
  required: boolean;
}

/** Every UDP port the server binds. Since 0.9 that's just the query port (verified: netstat shows one UDP socket). */
export function serverPorts(inst: ServerInstance): PortRow[] {
  return [{ port: inst.config.queryPort, label: 'Query / game', purpose: 'Steam server browser queries and player traffic – the only port players need', required: true }];
}

let netCache: { at: number; value: Promise<NetInfo> } | null = null;

/** Public IP + LAN addresses + gateway, cached for 5 minutes (the public IP lookup is an HTTP call). */
export function netInfo(force = false): Promise<NetInfo> {
  if (!force && netCache && Date.now() - netCache.at < 300_000) return netCache.value;
  const value = api.netInfo().catch((e) => {
    netCache = null;
    throw e;
  });
  netCache = { at: Date.now(), value };
  return value;
}

/** The LAN address players' traffic should be forwarded to. */
export function lanAddress(inst: ServerInstance, net: NetInfo | null): string | null {
  if (inst.config.ip && inst.config.ip !== '0.0.0.0') return inst.config.ip;
  return net?.local[0]?.ip ?? null;
}
