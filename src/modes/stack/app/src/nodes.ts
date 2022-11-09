import { Control, lndControls, relayControls, proxyControls } from "./controls";

type Types =
  | "bitcoind"
  | "lnd"
  | "proxy"
  | "relay"
  | "tribes"
  | "meme"
  | "mqtt"
  | "auth"
  | "postgres";

export interface Node {
  name: string;
  type: Types;
  x: number;
  y: number;
  outs: string[];
  controls?: Control[];
  data: string;
}

const nodes: Node[] = [
  {
    name: "Bitcoind",
    type: "bitcoind",
    x: 50,
    y: 150,
    outs: ["LND"],
    data: "",
  },
  {
    name: "LND",
    type: "lnd",
    x: 280,
    y: 200,
    outs: ["Proxy"],
    controls: lndControls,
    data: "2 channels",
  },
  {
    name: "Proxy",
    type: "proxy",
    x: 480,
    y: 201,
    outs: ["Relay"],
    controls: proxyControls,
    data: "56 users",
  },
  {
    name: "Relay",
    type: "relay",
    x: 700,
    y: 260,
    outs: [],
    controls: relayControls,
    data: "2 clients",
  },
];
export default nodes;
