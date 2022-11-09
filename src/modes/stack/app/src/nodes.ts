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

export interface Node {
  name: string;
  type: NodeType;
  place: Place;
  links?: string[];
  network?: Network;
  [key: string]: string | string[];
}

interface Config {
  network: Network;
  nodes: Node[];
}

const config: Config = {
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
      links: ["proxy1", "lnd1"],
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

export { config };

// export interface Node {
//   name: string;
//   type: Types;
//   x: number;
//   y: number;
//   outs: string[];
//   controls?: Control[];
//   data: string;
// }

// const nodes: Node[] = [
//   {
//     name: "Bitcoind",
//     type: "bitcoind",
//     x: 50,
//     y: 150,
//     outs: ["LND"],
//     data: "",
//   },
//   {
//     name: "LND",
//     type: "lnd",
//     x: 280,
//     y: 200,
//     outs: ["Proxy"],
//     controls: lndControls,
//     data: "2 channels",
//   },
//   {
//     name: "Proxy",
//     type: "proxy",
//     x: 480,
//     y: 201,
//     outs: ["Relay"],
//     controls: proxyControls,
//     data: "56 users",
//   },
//   {
//     name: "Relay",
//     type: "relay",
//     x: 700,
//     y: 260,
//     outs: [],
//     controls: relayControls,
//     data: "2 clients",
//   },
// ];
// export default nodes;
