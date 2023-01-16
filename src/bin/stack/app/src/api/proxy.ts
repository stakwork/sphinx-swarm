import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

export interface ProxyBalance {
  total: number;
  user_count: number;
}

async function proxyCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Proxy", { cmd, content }, tag);
}

export async function get_proxy_balances(
  tag: string
) {
    return await proxyCmd("GetBalance", tag);
}
