const IS_DEV = window.location.host === "localhost:5173" || "127.0.0.1:5173";

let root = "/api";
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
  | "CreateChannel"
  | "GetBalance"
  | "NewAddress"
  | "ListWallets";

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
