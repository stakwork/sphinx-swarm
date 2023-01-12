export interface Stack {
  network: Network;
  nodes: Node[];
}

export interface Node {
  name: string;
  type: NodeType;
  place: Place;
  version?: string; // Internal always have version
  links?: string[];
  network?: Network;
  url?: string;
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
  | "Cache";

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
];

type Place = "Internal" | "External";

type Network = "bitcoin" | "regtest";

const stack: Stack = {
  network: "regtest",
  nodes: [
    {
      place: "Internal",
      type: "Btc",
      version: "23.0",
      name: "bitcoind",
      network: "regtest",
      user: "user",
      pass: "",
    },
    {
      place: "Internal",
      type: "Lnd",
      name: "lnd1",
      version: "v0.14.3-beta.rc1",
      network: "regtest",
      port: "10009",
      http_port: "8881",
      links: ["bitcoind"],
    },
    {
      place: "Internal",
      type: "Proxy",
      name: "proxy1",
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
      name: "relay1",
      port: "3000",
      links: ["proxy1", "lnd1", "tribes", "memes"],
    },
    {
      place: "Internal",
      type: "Traefik",
      version: "v2.2.1",
      name: "reverse-proxy",
      links: ["lnd1", "relay1"],
    },
    {
      place: "Internal",
      type: "Cache",
      version: "0.1.14",
      name: "cache1",
      links: ["lnd1", "tribes"],
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

const defaultPositions = [
  [100, 100],
  [370, 200],
  [660, 130],
  [920, 350],
  [920, 40],
  [100, 250],
  [260, 400],
  [560, 500],
];

export { stack, defaultPositions };
