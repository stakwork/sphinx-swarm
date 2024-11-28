import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

// test route hint
declare global {
  interface Window {
    route_hint: string;
    maxfeepercent: number;
    exemptfee: number;
    tlvs: { [k: number]: number[] };
  }
}

async function clnCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Cln", { cmd, content }, tag);
}

export async function get_info(tag: string) {
  return await clnCmd("GetInfo", tag);
}

export async function list_peers(tag: string) {
  return await clnCmd("ListPeers", tag);
}

export async function list_peer_channels(tag: string) {
  return await clnCmd("ListPeerChannels", tag);
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

export async function keysend(
  tag: string,
  dest: string,
  amt: number,
  route_hint?: string,
  maxfeepercent?: number,
  exemptfee?: number
) {
  const bod: { [k: string]: any } = {
    dest,
    amt,
  };
  if (route_hint) bod.route_hint = route_hint;
  if (maxfeepercent) bod.maxfeepercent = maxfeepercent;
  if (exemptfee) bod.exemptfee = exemptfee;
  return await clnCmd("PayKeysend", tag, bod);
}

export async function close_channel(
  tag: string,
  id: string,
  destination: string
) {
  return await clnCmd("CloseChannel", tag, { id, destination });
}

export async function list_invoices(tag: string, payment_hash?: string) {
  return await clnCmd("ListInvoices", tag, payment_hash && { payment_hash });
}

export async function list_pays(tag: string, payment_hash?: string) {
  return await clnCmd("ListPays", tag, payment_hash && { payment_hash });
}

export async function create_channel(
  tag: string,
  pubkey: string,
  amount: number,
  satsperbyte: number
) {
  return await clnCmd("AddChannel", tag, { pubkey, amount, satsperbyte });
}

export async function add_peer(
  tag: string,
  pubkey: string,
  host: string,
  alias?: string
) {
  return await clnCmd("AddPeer", tag, { pubkey, host, alias });
}
