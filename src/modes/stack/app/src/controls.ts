type ControlType = "dropdown" | "number" | "text";
export interface Control {
  type: ControlType;
  name: string;
  value: any;
  items?: any[];
}

const relayControls: Control[] = [
  {
    name: "Thing One",
    value: "item1",
    type: "dropdown",
    items: [
      { id: "item1", text: "Item 1" },
      { id: "item2", text: "Item 2" },
    ],
  },
  { name: "Thing 2", type: "number", value: 42 },
  { name: "Thing 3", type: "text", value: "Some Text" },
];

const lndControls: Control[] = [
  { name: "LND 2", type: "number", value: 42 },
  { name: "LND 3", type: "text", value: "Some Text" },
  {
    name: "LND One",
    value: "item1",
    type: "dropdown",
    items: [
      { id: "item1", text: "blah blah" },
      { id: "item2", text: "soemthing" },
    ],
  },
];

const proxyControls: Control[] = [
  { name: "Proxy 3", type: "text", value: "Some Text" },
  {
    name: "Proxy One",
    value: "item1",
    type: "dropdown",
    items: [
      { id: "item1", text: "ASDFASDF" },
      { id: "item2", text: "QWERQWER" },
    ],
  },
  { name: "Proxy 2", type: "number", value: 42 },
];

const nodeItemTypes: Control[] = [
  {
    name: "Type",
    type: "dropdown",
    value: "",
    items: [
      {
        id: "node1",
        text: "Btc",
      },
      {
        id: "node2",
        text: "Lnd",
      },
      {
        id: "node3",
        text: "Proxy",
      },
      {
        id: "node4",
        text: "Relay",
      },
      {
        id: "node5",
        text: "Tribes",
      },
      {
        id: "node6",
        text: "Meme",
      },
      {
        id: "node7",
        text: "Auth",
      },
      {
        id: "node8",
        text: "Postgres",
      },
    ],
  },
  {
    name: "Name",
    type: "text",
    value: "",
  },
  {
    name: "Connections",
    type: "dropdown",
    value: "",
    items: [
      {
        id: "con1",
        text: "Btc",
      },
      {
        id: "con2",
        text: "Lnd",
      },
      {
        id: "con3",
        text: "Proxy",
      },
      {
        id: "con4",
        text: "Relay",
      },
    ]
  }
];

export const controls = {
  Relay: relayControls,
  Proxy: proxyControls,
  Lnd: lndControls,
  NodeTypes: nodeItemTypes,
};
