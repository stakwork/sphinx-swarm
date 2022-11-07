const IS_DEV = window.location.host === "localhost:8080";

let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
}

type CmdType = "Swarm" | "Relay";

export type Cmd = "GetConfig" | "ListUsers" | "AddUser";

interface CmdData {
  cmd: Cmd;
  content?: any;
}

export async function send_cmd(type: CmdType, data: CmdData) {
  const txt = JSON.stringify({ type, data });
  const r = await fetch(`${root}/cmd?txt=${txt}&tag=SWARM`);
  const result = await r.json();
  return result;
}

import * as swarm from "./swarm";
import * as relay from "./relay";

export { swarm, relay };
