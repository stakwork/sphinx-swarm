const IS_DEV = window.location.host === "localhost:8080";

let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
}

type CmdType = "Swarm" | "Relay" | "Bitcoind" | "Lnd";

export type Cmd =
  | "GetConfig"
  | "ListUsers"
  | "AddUser"
  | "GetInfo"
  | "GetContainerLogs"
  | "TestMine"
  | "ListChannels" 
  | "AddPeer"
  | "CreateChannel";

interface CmdData {
  cmd: Cmd;
  content?: any;
}

export async function send_cmd(type: CmdType, data: CmdData, tag?: string) {
  const txt = JSON.stringify({ type, data });
  const r = await fetch(`${root}/cmd?txt=${txt}&tag=${tag || "SWARM"}`);
  const result = await r.json();
  return result;
}
