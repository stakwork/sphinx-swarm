import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";

export interface Channel {
  active: boolean;
  remote_pubkey: string;
  channel_point: string;
  chan_id: number;
  capacity: number;
  local_balance: number;
  remote_balance: number;
  commit_fee: number;
  commit_weight: number;
  fee_per_kw: number;
  unsettled_balance: number;
  total_satoshis_sent: number;
  total_satoshis_received: number;
  num_updates: number;
  // pending_htlcs: Vec<Htlc>,
  csv_delay: number;
  private: number;
  initiator: number;
  chan_status_flags: string;
  local_chan_reserve_sat: number;
  remote_chan_reserve_sat: number;
  static_remote_key: boolean;
  commitment_type: number;
  lifetime: number;
  uptime: number;
  close_address: string;
  push_amount_sat: number;
  thaw_height: number;
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

export async function add_peer(tag: string, pubkey: string, host: string) {
  return await lndCmd("AddPeer", tag, { pubkey, host });
}

export async function create_channel(
  tag: string,
  pubkey: string,
  amount: number,
  satsperbyte
) {
  return await lndCmd("CreateChannel", tag, { pubkey, amount, satsperbyte });
}
