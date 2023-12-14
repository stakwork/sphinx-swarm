import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

export interface RelayBalance {
  reserve: number;
  full_balance: number;
  balance: number;
  pending_open_balance: number;
}

async function relayCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Relay", { cmd, content }, tag);
}

export async function list_users(tag: string) {
  return await relayCmd("ListUsers", tag);
}

export async function get_chats(tag: string) {
  return await relayCmd("GetChats", tag);
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

export async function get_auth_token(tag: string) {
  return await relayCmd("GetToken", tag);
}

export async function get_balance(tag: string) {
  return await relayCmd("GetBalance", tag);
}
