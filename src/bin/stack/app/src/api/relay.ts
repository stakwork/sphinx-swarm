import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

async function relayCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Relay", { cmd, content }, tag);
}

export async function list_users(tag: string) {
  return await relayCmd("ListUsers", tag);
}

export async function get_chats(tag: string) {
  return await relayCmd("GetChats", tag);
}

export async function create_tribe(tag: string, name: string) {
  return await relayCmd("CreateTribe", tag, { name });
}

export async function add_user(tag: string, initial_sats?: number) {
  return await relayCmd("AddUser", tag, {
    ...(initial_sats && { initial_sats }),
  });
}

export async function add_default_tribe(tag: string, id: number) {
  return await relayCmd("AddDefaultTribe", tag, { id });
}

export async function remove_default_tribe(tag: string, id: number) {
  return await relayCmd("RemoveDefaultTribe", tag, { id });
}
