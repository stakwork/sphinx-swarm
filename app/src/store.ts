import { writable, derived, type Readable } from "svelte/store";
import type { Node, Stack } from "./nodes";
import { initialUsers } from "./relay/users";
import type { User } from "./relay/users";
import type { Tribe, Person } from "./api/tribes";
import type { LndChannel, LndPeer } from "./api/lnd";
import type { BtcInfo } from "./api/btc";
import type { ProxyBalance } from "./api/proxy";
import { userKey, type TokenData } from "./api/cmd";
import { decode } from "js-base64";
import * as api from "./api";
import type { RelayBalance } from "./api/relay";
import type { Container } from "./api/swarm";

export const emptyStack: Stack = {
  network: "regtest",
  nodes: [],
  ready: false,
};

export const selectedNode = writable<Node>();

export const stack = writable<Stack>(emptyStack);

export const users = writable<User[]>(initialUsers);

export const current_swarm_user = writable<SwarmUser>();

export const tribes = writable<Tribe>({
  page: 1,
  total: 0,
  data: [],
});

export const people = writable<Person[]>([]);

export const channels = writable<{ [tag: string]: LndChannel[] }>({});

export const proxy = writable<ProxyBalance>({
  total: 0,
  user_count: 0,
});

export const walletBalance = writable<number>(0);

export const lightningAddresses = writable<{ [tag: string]: string }>({});

export const btcinfo = writable<BtcInfo>();

export const peers = writable<{ [tag: string]: LndPeer[] }>({});

export const lndBalances = writable<{ [tag: string]: number }>({});

export const unconfirmedBalance = writable<{ [tag: string]: number }>({});

export const relayBalances = writable<{ [tag: string]: RelayBalance }>({});

export const activeInvoice = writable<{ [tag: string]: string }>({});

export const activeUser = writable<string>();

export const containers = writable<Container[]>([]);

export const exitedNodes = writable<string[]>([]);

export const onChainAddressGeneratedForOnboarding = writable<boolean>(false);

export const copiedAddressForOnboarding = writable<boolean>(false);

export const pendingTransaction = writable<boolean>(false);

export const createdPeerForOnboarding = writable<boolean>(false);

export const channelCreatedForOnboarding = writable<boolean>(false);

export const adminIsCreatedForOnboarding = writable<boolean>(false);

export const isOnboarding = writable<boolean>(false);

export const boltwallSuperAdminPubkey = writable<string>("");

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

export interface ChannelBalances {
  inbound: number;
  outbound: number;
}
export function makeChannelBalances(
  channels: { [tag: string]: LndChannel[] },
  selectedNode: Node
): ChannelBalances {
  if (!(selectedNode && selectedNode.name)) {
    return { inbound: 0, outbound: 0 };
  }
  const tag = selectedNode.name;
  if (!channels[tag]) return { inbound: 0, outbound: 0 };
  return {
    inbound:
      channels[tag] && channels[tag].length
        ? channels[tag].reduce((acc, chan) => {
            return chan.active ? acc + chan.remote_balance : acc;
          }, 0)
        : 0,
    outbound:
      channels[tag] && channels[tag].length
        ? channels[tag].reduce((acc, chan) => {
            return chan.active ? acc + chan.local_balance : acc;
          }, 0)
        : 0,
  };
}

export const channelBalances = derived(
  [channels, selectedNode],
  ([$channels, $selectedNode]) => makeChannelBalances($channels, $selectedNode)
);

export const finishedOnboarding = derived(
  [channels, users, lndBalances, peers],
  ([$channels, $users, $lndBalances, $peers]) => {
    let hasChannels = false;
    let hasBalance = false;
    let hasPeers = false;
    let hasUsers = false;
    for (let key in $channels) {
      if ($channels[key].length > 0) {
        hasChannels = true;
      }
    }

    for (let key in $peers) {
      if ($peers[key].length > 0) {
        hasPeers = true;
      }
    }

    for (let key in $lndBalances) {
      if ($lndBalances[key] > 0) {
        hasBalance = true;
      }
    }
    const hasAdmin = $users?.find((user) => user.is_admin && user.alias);
    if (hasAdmin && $users.length > 1) hasUsers = true;
    return { hasAdmin, hasChannels, hasBalance, hasPeers, hasUsers };
  }
);

function nodeHostLocalhost(node: Node): string {
  if (!node) return;
  if (node.type === "Relay") {
    return `localhost:${node.port || "3000"}`;
  } else if (node.type === "Lnd") {
    return `localhost:${node.rpc_port || "10009"}`;
  } else if (node.type === "Cln") {
    return `localhost:${node.grpc_port || "10009"}`;
  } else if (node.type === "Proxy") {
    return `localhost:${node.port || "10009"}`;
  }
  return "localhost";
}

export const node_host = derived(
  [stack, selectedNode],
  ([$stack, $selectedNode]) => {
    if (!$selectedNode) return "";
    return $stack.host
      ? `${$selectedNode.name}.${$stack.host}`
      : nodeHostLocalhost($selectedNode);
  }
);

export type NodeState = "restarting" | "running" | "exited" | undefined;

export const node_state: Readable<NodeState> = derived(
  [selectedNode, containers],
  ([$selectedNode, $containers]) => {
    if (!$selectedNode) return;
    if ($selectedNode.place === "External") return;
    if (!$containers) return;
    if (!Array.isArray($containers)) return;
    const con = $containers?.find((n) =>
      n.Names.includes(`/${$selectedNode.name}.sphinx`)
    );
    if (!con) return;
    return con.State as NodeState;
  }
);

export const nodes_exited = derived([containers], ([$containers]) => {
  let exitedArray = [];

  for (let con of $containers) {
    if (con.State === "exited") {
      let nameArray = con.Names[0].split("/");
      let name = nameArray[1].replace(".sphinx", "");

      exitedArray = [...exitedArray, name];
    }
  }

  return exitedArray;
});

export const saveUserToStore = async (user: string = "") => {
  if (user) {
    localStorage.setItem(userKey, user);
    return activeUser.set(user);
  }

  let storageUser = localStorage.getItem(userKey);

  if (storageUser) {
    const jwts = storageUser.split(".");

    const decodedData: TokenData = JSON.parse(decode(jwts[1]));

    if (decodedData.exp * 1000 > Date.now()) {
      const refresh = await api.swarm.refresh_token(storageUser);

      // save the new token to localstorage
      localStorage.setItem(userKey, refresh.token);
      return activeUser.set(refresh.token);
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

export async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const hsmd = writable<boolean>(false);

export interface HsmdClients {
  pubkey?: string;
  current?: string;
  clients: { [k: string]: any };
}

type SwarmUserRole = "Admin" | "SubAdmin" | "Super";

export interface SwarmUser {
  id: number;
  username: string;
  pubkey: string;
  role: SwarmUserRole;
}

export const hsmdClients = writable<HsmdClients>();
