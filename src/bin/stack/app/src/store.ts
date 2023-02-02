import { writable, derived } from "svelte/store";
import type { Node, Stack } from "./nodes";
import { initialUsers } from "./relay/users";
import type { User } from "./relay/users";
import type { Tribe, Person } from "./api/tribes";
import type { Channel, Peer } from "./api/lnd";
import type { BtcInfo } from "./api/btc";
import type { ProxyBalance } from "./api/proxy";
import { userKey, type TokenData } from "./api/cmd";
import { decode } from "js-base64";

export const emptyStack: Stack = { network: "regtest", nodes: [] };

export const selectedNode = writable<Node>();

export const stack = writable<Stack>(emptyStack);

export const users = writable<User[]>(initialUsers);

export const tribes = writable<Tribe>({
  page: 1,
  total: 0,
  data: [],
});

export const people = writable<Person[]>([]);

export const channels = writable<{ [tag: string]: Channel[] }>({});

export const proxy = writable<ProxyBalance>({
  total: 0,
  user_count: 0,
});

export const walletBalance = writable<number>(0);

export const lightningAddresses = writable<{ [tag: string]: string }>({});

export const btcinfo = writable<BtcInfo>();

export const peers = writable<{ [tag: string]: Peer[] }>({});

export const nodeBalances = writable<{ [tag: string]: number }>({});

export const activeInvoice = writable<{ [tag: string]: string }>({});

export const activeUser = writable<string>();

export const balances = derived(
  [channels, selectedNode],
  ([$channels, $selectedNode]) => {
    if (!($selectedNode && $selectedNode.name)) {
      return { inbound: 0, outbound: 0 };
    }
    const tag = $selectedNode.name;
    return {
      inbound:
        $channels[tag] && $channels[tag].length
          ? $channels[tag].reduce((acc, chan) => acc + chan.remote_balance, 0)
          : 0,
      outbound:
        $channels[tag] && $channels[tag].length
          ? $channels[tag].reduce((acc, chan) => acc + chan.local_balance, 0)
          : 0,
    };
  }
);

export const node_host = derived(
  [stack, selectedNode],
  ([$stack, $selectedNode]) => {
    return $selectedNode && $stack.host
      ? `${$selectedNode.name}.${$stack.host}`
      : "localhost";
  }
);

export const saveUserToStore = (user: string = "") => {
  if (user) {
    localStorage.setItem(userKey, user);
    return activeUser.set(user);
  }

  let storageUser = localStorage.getItem(userKey);

  if (storageUser) {
    const jwts = storageUser.split(".");

    const decodedData: TokenData = JSON.parse(decode(jwts[1]));

    if (decodedData.exp * 1000 > Date.now()) {
      return activeUser.set(storageUser);
    }
  }
};

export const logoutUser = () => {
  localStorage.setItem(userKey, "");
  return activeUser.set("");
};

/*
 * Call to get user token from localstorage
 * and save to store
 */
saveUserToStore();
