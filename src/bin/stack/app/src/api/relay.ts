import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

async function relayCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Relay", { cmd, content }, tag);
}

export async function list_users(tag: string) {
  return await relayCmd("ListUsers", tag);
}

export async function add_user(tag: string) {
  return await relayCmd("AddUser", tag);
}
