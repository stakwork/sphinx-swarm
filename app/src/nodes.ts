export interface Stack {
  network: Network;
  nodes: Node[];
  host?: string;
}

export interface Node {
  name: string;
  type: NodeType;
  place: Place;
  version?: string; // Internal always have version
  links?: string[];
  network?: Network;
  url?: string;
  host?: string;
  // any other optional field
  [key: string]: string | string[];
}

export type NodeType =
  | "Btc"
  | "Lnd"
  | "Proxy"
  | "Relay"
  | "Tribes"
  | "Meme"
  | "Mqtt"
  | "Auth"
  | "Postgres"
  | "Traefik"
  | "Cache"
  | "Jarvis"
  | "BoltWall"
  | "Neo4j"
  | "NavFiber"
  | "Cln";

export const allNodeTypes: NodeType[] = [
  "Btc",
  "Lnd",
  "Proxy",
  "Relay",
  "Tribes",
  "Meme",
  "Mqtt",
  "Auth",
  "Postgres",
  "Traefik",
  "Cache",
  "BoltWall",
  "Neo4j",
  "NavFiber",
  "Jarvis",
];

export const upgradableNodes: NodeType[] = [
  "Btc",
  "Lnd",
  "Proxy",
  "Relay",
  "Cache",
  "Cln",
  "Jarvis",
];

type Place = "Internal" | "External";

type Network = "bitcoin" | "regtest";

const stack: Stack = {
  network: "regtest",
  nodes: [
    {
      place: "Internal",
      type: "Btc",
      version: "v23.0",
      name: "bitcoind",
      network: "regtest",
      user: "user",
      pass: "",
    },
    {
      place: "Internal",
      type: "Lnd",
      name: "lnd",
      version: "v0.14.3-beta.rc1",
      network: "regtest",
      port: "10009",
      http_port: "8881",
      links: ["bitcoind"],
    },
    {
      place: "Internal",
      type: "Proxy",
      name: "proxy",
      version: "0.1.2",
      network: "regtest",
      port: "11111",
      admin_port: "5050",
      admin_token: "",
      store_key: "",
      new_nodes: null,
      links: ["lnd1"],
    },
    {
      place: "Internal",
      type: "Relay",
      version: "v2.2.12",
      name: "relay",
      port: "3000",
      links: ["proxy", "lnd", "tribes", "memes", "cache"],
    },
    {
      place: "Internal",
      type: "Traefik",
      version: "v2.2.1",
      name: "load_balancer",
      links: ["lnd", "relay"],
    },
    {
      place: "Internal",
      type: "Cache",
      version: "0.1.14",
      name: "cache",
      links: ["lnd", "tribes"],
    },
    {
      name: "tribes",
      place: "External",
      type: "Tribes",
      url: "tribes.sphinx.chat",
    },
    {
      name: "memes",
      place: "External",
      type: "Meme",
      url: "meme.sphinx.chat",
    },
  ],
};

const xOffset = -220;
const yOffset = -70;
function offset(xy) {
  return [xy[0] + xOffset, xy[1] + yOffset];
}
function defaultPositions() {
  return Object.fromEntries(
    Object.entries(defpos).map(([k, v], i) => [k, offset(v)])
  );
}

const defpos = {
  bitcoind: [320, 140],
  lnd: [580, 200],
  cln: [580, 200],
  cln_1: [680, 140],
  cln_2: [680, 280],
  lnd_1: [650, 420],
  proxy: [850, 140],
  relay: [1150, 375],
  load_balancer: [895, 40],
  cache: [900, 600],
  tribes: [680, 650],
  memes: [900, 720],
  jarvis: [750, 475],
  boltwall: [750, 375],
  neo4j: [480, 425],
  navfiber: [1150, 475],
};

export { stack, defaultPositions };
