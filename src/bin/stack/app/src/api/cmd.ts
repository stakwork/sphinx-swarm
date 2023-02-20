const IS_DEV =
  window.location.host === "localhost:5173" ||
  window.location.host === "127.0.0.1:5173";

export let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
}

type CmdType = "Swarm" | "Relay" | "Bitcoind" | "Lnd" | "Proxy";

export type Cmd =
  | "GetConfig"
  | "ListUsers"
  | "AddUser"
  | "GetInfo"
  | "GetContainerLogs"
  | "TestMine"
  | "ListChannels"
  | "AddPeer"
  | "ListPeers"
  | "AddChannel"
  | "GetBalance"
  | "NewAddress"
  | "ListWallets"
  | "ListVersions"
  | "UpdateInstance"
  | "AddInvoice"
  | "PayInvoice"
  | "PayKeysend"
  | "AddDefaultTribe"
  | "RemoveDefaultTribe"
  | "GetChats"
  | "CreateTribe"
  | "GetToken"
  | "ListContainers"
  | "StartContainer"
  | "StopContainer"
  | "UpdateNode";

interface CmdData {
  cmd: Cmd;
  content?: any;
}

export interface TokenData {
  exp: number;
  user: number;
}

export const userKey = "SPHINX_TOKEN";

export async function send_cmd(type: CmdType, data: CmdData, tag?: string) {
  const txt = JSON.stringify({ type, data });
  try {
    const r = await fetch(`${root}/cmd?txt=${txt}&tag=${tag || "SWARM"}`, {
      headers: {
        "x-jwt": localStorage.getItem(userKey),
      },
    });
    return await r.json();
  } catch (e) {
    console.error(e);
  }
}
