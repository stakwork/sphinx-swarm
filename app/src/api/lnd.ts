import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

export interface LndInfo {
  identity_pubkey: string;
}
export interface LndChannel {
  active: boolean;
  remote_pubkey: string;
  channel_point: string;
  chan_id: string;
  capacity: number;
  local_balance: number;
  remote_balance: number;
  commit_fee?: number;
  commit_weight?: number;
  fee_per_kw?: number;
  unsettled_balance?: number;
  total_satoshis_sent?: number;
  total_satoshis_received?: number;
  num_updates?: number;
  // pending_htlcs: Vec<Htlc>,
  csv_delay?: number;
  private?: number;
  initiator?: number;
  chan_status_flags?: string;
  local_chan_reserve_sat?: number;
  remote_chan_reserve_sat?: number;
  static_remote_key?: boolean;
  commitment_type?: number;
  lifetime?: number;
  uptime?: number;
  close_address?: string;
  push_amount_sat?: number;
  thaw_height?: number;
}

export interface LndPeer {
  pub_key: string;
  address: string;
  bytes_sent: number;
  bytes_recv: number;
  sat_sent: number;
  sat_recv: number;
  inbound: number;
  ping_time: number;
  sync_type: number;
}

async function lndCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Lnd", { cmd, content }, tag);
}

export async function get_info(tag: string) {
  return await lndCmd("GetInfo", tag);
}

export async function list_channels(tag: string) {
  return await lndCmd("ListChannels", tag);
}

export async function add_peer(
  tag: string,
  pubkey: string,
  host: string,
  alias?: string
) {
  return await lndCmd("AddPeer", tag, { pubkey, host, alias });
}

export async function list_peers(tag: string) {
  return await lndCmd("ListPeers", tag);
}

export async function get_balance(tag: string) {
  return await lndCmd("GetBalance", tag);
}

export async function list_pending_channels(tag: string) {
  return await lndCmd("ListPendingChannels", tag);
}

export async function create_channel(
  tag: string,
  pubkey: string,
  amount: number,
  satsperbyte: number
) {
  return await lndCmd("AddChannel", tag, { pubkey, amount, satsperbyte });
}

export async function new_address(tag: string) {
  return await lndCmd("NewAddress", tag);
}

export async function add_invoice(tag: string, amt_paid_sat) {
  return await lndCmd("AddInvoice", tag, { amt_paid_sat });
}

export async function pay_invoice(tag: string, payment_request) {
  return await lndCmd("PayInvoice", tag, { payment_request });
}

export async function keysend(
  tag: string,
  dest: string,
  amt: number,
  tlvs?: { [k: number]: number[] }
) {
  const body: any = { dest, amt };
  if (tlvs) body.tlvs = tlvs;
  return await lndCmd("PayKeysend", tag, body);
}

export async function list_invoices(tag: string) {
  return await lndCmd("ListInvoices", tag);
}

export async function list_payments(tag: string) {
  return await lndCmd("ListPayments", tag);
}
