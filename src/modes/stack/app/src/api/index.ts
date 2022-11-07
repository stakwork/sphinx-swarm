const IS_DEV = window.location.host === "localhost:8080";

let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
}

type CmdType = "Swarm" | "Relay";

export async function send_cmd(type: CmdType, data: any) {
  const txt = JSON.stringify({ type, data });
  console.log("send ", txt);
  const r = await fetch(`${root}/cmd?txt=${txt}&tag=SWARM`);
  const result = await r.json();
  console.log(result);
}

import * as swarm from "./swarm";

export { swarm };
