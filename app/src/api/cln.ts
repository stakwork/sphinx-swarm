import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

async function clnCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("CLN", { cmd, content }, tag);
}

export async function get_info(tag: string) {
  return await clnCmd("GetInfo", tag);
}

export async function list_peers(tag: string) {
  return await clnCmd("ListPeers", tag);
}

export async function list_funds(tag: string) {
  return await clnCmd("ListFunds", tag);
}

export async function new_address(tag: string) {
  return await clnCmd("NewAddress", tag);
}

export async function add_invoice(tag: string, amt_paid_sat) {
  return await clnCmd("AddInvoice", tag, { amt_paid_sat });
}

export async function pay_invoice(tag: string, payment_request) {
  return await clnCmd("PayInvoice", tag, { payment_request });
}

export async function keysend(tag: string, dest: string, amt: number) {
  return await clnCmd("PayKeysend", tag, { dest, amt });
}
