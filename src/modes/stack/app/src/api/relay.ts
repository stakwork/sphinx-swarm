import { send_cmd, Cmd } from "./cmd";

async function relayCmd(cmd: Cmd, content?: any) {
  return await send_cmd("Relay", { cmd, content });
}

export async function list_users() {
  return await relayCmd("ListUsers");
}

export async function add_user() {
  return await relayCmd("AddUser");
}
