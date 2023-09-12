import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

export interface ProxyBalance {
  total: number;
  user_count: number;
}

async function hsmdCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Hsmd", { cmd, content }, tag);
}

export async function get_clients(tag: string) {
  return await hsmdCmd("GetClients", tag);
}
