import { send_cmd, Cmd } from "./cmd";

async function lndCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Lnd", { cmd, content }, tag);
}

export async function get_info(tag: string) {
  return await lndCmd("GetInfo", tag);
}
