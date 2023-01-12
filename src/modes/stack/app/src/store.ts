import { writable, derived } from "svelte/store";
import { localStorageStore } from "./storage";
import { stack as initialStack } from "./nodes";
import type { Node, Stack } from "./nodes";
import { initialUsers } from "./relay/users";
import type { User } from "./relay/users";
import type { Tribe, Person } from "./api/tribes";
import type { Channel } from "./api/lnd";
import type { BtcInfo } from "./api/btc";
import type { ProxyBalance } from "./api/proxy";

export const selectedNode = writable<Node>();

export const stack = writable<Stack>(initialStack);

export const users = writable<User[]>(initialUsers);

export const tribes = writable<Tribe[]>([]);

export const people = writable<Person[]>([]);

export const channels = writable<Channel[]>([]);

export const proxy = writable<ProxyBalance>({
  total: 0,
  user_count: 0,
});

export const walletBalance = writable<number>(0);

export const lightningAddresses = writable<{ [tag: string]: string }>({});

export const balances = derived(channels, ($channels) => ({
  inbound:
    $channels && $channels.length
      ? $channels.reduce((acc, chan) => acc + chan.remote_balance, 0)
      : 0,
  outbound:
    $channels && $channels.length
      ? $channels.reduce((acc, chan) => acc + chan.local_balance, 0)
      : 0,
}));

export const btcinfo = writable<BtcInfo>();
