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
  "BoltWall",
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

export const chipSVG = `<svg version="1.1" height="22" width="22" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" x="0px" y="0px"
viewBox="0 0 512 512" style="enable-background:new 0 0 512 512;" xml:space="preserve" fill="#ddd">
<path d="M153.6,76.8h204.8c42.43,0,76.8,34.4,76.8,76.8v204.8c0,42.43-34.38,76.8-76.8,76.8H153.6c-42.4,0-76.8-34.38-76.8-76.8
 V153.6C76.8,111.2,111.2,76.8,153.6,76.8z M153.6,128c-14.13,0-25.6,11.48-25.6,25.6v204.8c0,14.15,11.48,25.6,25.6,25.6h204.8
 c14.15,0,25.6-11.45,25.6-25.6V153.6c0-14.13-11.45-25.6-25.6-25.6H153.6z M204.8,179.2h102.4c14.15,0,25.6,11.48,25.6,25.6v102.4
 c0,14.15-11.45,25.6-25.6,25.6H204.8c-14.13,0-25.6-11.45-25.6-25.6V204.8C179.2,190.68,190.68,179.2,204.8,179.2z M230.4,230.4
 v51.2h51.2v-51.2H230.4z M153.6,0c14.15,0,25.6,11.48,25.6,25.6v51.2c0,14.15-11.45,25.6-25.6,25.6c-14.13,0-25.6-11.45-25.6-25.6
 V25.6C128,11.48,139.48,0,153.6,0z M25.6,128h51.2c14.15,0,25.6,11.48,25.6,25.6c0,14.15-11.45,25.6-25.6,25.6H25.6
 C11.48,179.2,0,167.75,0,153.6C0,139.48,11.48,128,25.6,128z M435.2,128h51.2c14.15,0,25.6,11.48,25.6,25.6
 c0,14.15-11.45,25.6-25.6,25.6h-51.2c-14.13,0-25.6-11.45-25.6-25.6C409.6,139.48,421.08,128,435.2,128z M25.6,230.4h51.2
 c14.15,0,25.6,11.48,25.6,25.6c0,14.15-11.45,25.6-25.6,25.6H25.6C11.48,281.6,0,270.15,0,256C0,241.88,11.48,230.4,25.6,230.4z
  M435.2,230.4h51.2c14.15,0,25.6,11.48,25.6,25.6c0,14.15-11.45,25.6-25.6,25.6h-51.2c-14.13,0-25.6-11.45-25.6-25.6
 C409.6,241.88,421.08,230.4,435.2,230.4z M25.6,332.8h51.2c14.15,0,25.6,11.48,25.6,25.6c0,14.15-11.45,25.6-25.6,25.6H25.6
 C11.48,384,0,372.55,0,358.4C0,344.27,11.48,332.8,25.6,332.8z M435.2,332.8h51.2c14.15,0,25.6,11.48,25.6,25.6
 c0,14.15-11.45,25.6-25.6,25.6h-51.2c-14.13,0-25.6-11.45-25.6-25.6C409.6,344.27,421.08,332.8,435.2,332.8z M153.6,409.6
 c14.15,0,25.6,11.48,25.6,25.6v51.2c0,14.15-11.45,25.6-25.6,25.6c-14.13,0-25.6-11.45-25.6-25.6v-51.2
 C128,421.08,139.48,409.6,153.6,409.6z M358.4,0C372.55,0,384,11.48,384,25.6v51.2c0,14.15-11.45,25.6-25.6,25.6
 c-14.13,0-25.6-11.45-25.6-25.6V25.6C332.8,11.48,344.27,0,358.4,0z M358.4,409.6c14.15,0,25.6,11.48,25.6,25.6v51.2
 c0,14.15-11.45,25.6-25.6,25.6c-14.13,0-25.6-11.45-25.6-25.6v-51.2C332.8,421.08,344.27,409.6,358.4,409.6z M256,0
 c14.15,0,25.6,11.48,25.6,25.6v51.2c0,14.15-11.45,25.6-25.6,25.6c-14.13,0-25.6-11.45-25.6-25.6V25.6C230.4,11.48,241.88,0,256,0z
  M256,409.6c14.15,0,25.6,11.48,25.6,25.6v51.2c0,14.15-11.45,25.6-25.6,25.6c-14.13,0-25.6-11.45-25.6-25.6v-51.2
 C230.4,421.08,241.88,409.6,256,409.6z"/>
</svg>`;
