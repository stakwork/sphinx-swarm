import { send_cmd, Cmd } from ".";

async function relayCmd(cmd: Cmd, content?: any) {
  return await send_cmd("Relay", { cmd, content });
}

export async function list_users() {
  return await relayCmd("ListUsers");
}
