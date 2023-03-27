import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

async function clnCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("CLN", { cmd, content }, tag);
}

export async function get_info(tag: string) {
  return await clnCmd("GetInfo", tag);
}
