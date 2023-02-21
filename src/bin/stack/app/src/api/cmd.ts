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
  | "StopContainer";

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
  let ret = "";
  try {
    const r = await fetch(`${root}/cmd?txt=${txt}&tag=${tag || "SWARM"}`, {
      headers: {
        "x-jwt": localStorage.getItem(userKey),
      },
    });
    ret = await r.text();
    const jj = JSON.parse(ret);
    if (jj["stack_error"]) {
      return console.warn("=> cmd error:", jj["stack_error"]);
    }
    return jj;
  } catch (e) {
    console.warn("=> cmd error:", ret);
  }
}
