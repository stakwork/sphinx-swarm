const IS_DEV =
  window.location.host === "localhost:5173" ||
  window.location.host === "127.0.0.1:5173";

export let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
}
console.log("=> root api url:", root);

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
  | "CreateTribe";

interface CmdData {
  cmd: Cmd;
  content?: any;
}

export async function send_cmd(type: CmdType, data: CmdData, tag?: string) {
  const txt = JSON.stringify({ type, data });
  try {
    const r = await fetch(`${root}/cmd?txt=${txt}&tag=${tag || "SWARM"}`);
    return await r.json();
  } catch (e) {
    console.error(e);
  }
}
