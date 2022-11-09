type ControlType = "dropdown" | "number" | "text";
export interface Control {
  type: ControlType;
  name: string;
  value: any;
  items?: any[];
}

export const relayControls: Control[] = [
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
export const lndControls: Control[] = [
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
export const proxyControls: Control[] = [
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
