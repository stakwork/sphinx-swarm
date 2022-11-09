export interface Stack {
  network: Network;
  nodes: Node[];
}

export interface Node {
  name: string;
  type: NodeType;
  place: Place;
  links?: string[];
  network?: Network;
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
  | "Postgres";

type Place = "Internal" | "External";

type Network = "bitcoin" | "regtest";

const stack: Stack = {
  network: "regtest",
  nodes: [
    {
      place: "Internal",
      type: "Btc",
      name: "bitcoind",
      network: "regtest",
      user: "user",
      pass: "",
    },
    {
      place: "Internal",
      type: "Lnd",
      name: "lnd1",
      network: "regtest",
      port: "10009",
      http_port: "8881",
      links: ["bitcoind"],
    },
    {
      place: "Internal",
      type: "Proxy",
      name: "proxy1",
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
      name: "relay1",
      port: "3000",
      links: ["proxy1", "lnd1", "tribes", "memes"],
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
  [50, 50],
  [220, 150],
  [360, 80],
  [520, 300],
  [160, 350],
  [360, 450],
];

export { stack, defaultPositions };
