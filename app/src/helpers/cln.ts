import { bufferToHexString, convertMillisatsToSats } from "./";
import long from "long";

enum GreenlightChannelState {
  CHANNELD_AWAITING_LOCKIN = "CHANNELD_AWAITING_LOCKIN",
  /* Normal operating state. */
  CHANNELD_NORMAL = "CHANNELD_NORMAL",
  /* We are closing, pending HTLC resolution. */
  CHANNELD_SHUTTING_DOWN = "CHANNELD_SHUTTING_DOWN",
  /* Exchanging signatures on closing tx. */
  CLOSINGD_SIGEXCHANGE = "CLOSINGD_SIGEXCHANGE",
  /* Waiting for onchain event. */
  CLOSINGD_COMPLETE = "CLOSINGD_COMPLETE",
  /* Waiting for unilateral close to hit blockchain. */
  AWAITING_UNILATERAL = "AWAITING_UNILATERAL",
  /* We've seen the funding spent, we're waiting for onchaind. */
  FUNDING_SPEND_SEEN = "FUNDING_SPEND_SEEN",
  /* On chain */
  ONCHAIN = "ONCHAIN",
  /* Final state after we have fully settled on-chain */
  CLOSED = "CLOSED",
  /* For dual-funded channels, we start at a different state.
   * We transition to 'awaiting lockin' after sigs have
   * been exchanged */
  DUALOPEND_OPEN_INIT = "DUALOPEND_OPEN_INIT",
  /* Dual-funded channel, waiting for lock-in */
  DUALOPEND_AWAITING_LOCKIN = "DUALOPEND_AWAITING_LOCKIN",
}

export function parseClnGetInfo(res) {
  const pubkey = bufferToHexString(res.id);
  return { identity_pubkey: pubkey };
}

export function parseClnListPeerRes(res: {
  peers: { id: Buffer; netaddr; channels }[];
}) {
  // pub_key: string;
  // address: string;
  // bytes_sent: number;
  // bytes_recv: number;
  // sat_sent: number;
  // sat_recv: number;
  // inbound: number;
  // ping_time: number;
  // sync_type: number;
  let channels: {
    remote_pubkey: string;
    capacity: number;
    local_balance: number;
    remote_balance: number;
    channel_point: string;
    active: boolean;
  }[] = [];
  const peers = res.peers.map((peer) => {
    const pub_key = bufferToHexString(peer.id);
    channels = [...channels, ...parseClnChannelList(peer.channels, pub_key)];
    return {
      pub_key,
      address: peer.netaddr[0],
      bytes_recv: 0,
      bytes_sent: 0,
      sat_sent: 0,
      sat_recv: 0,
      inbound: 0,
      ping_time: 0,
      sync_type: 0,
    };
  });
  return { peers, channels };
}

function parseClnChannelList(channels: any, pubkey: string) {
  // active: boolean;
  // remote_pubkey: string;
  // channel_point: string;
  // chan_id: number;
  // capacity: number;
  // local_balance: number;
  // remote_balance: number;
  // commit_fee: number;
  // commit_weight: number;
  // fee_per_kw: number;
  // unsettled_balance: number;
  // total_satoshis_sent: number;
  // total_satoshis_received: number;
  // num_updates: number;
  // // pending_htlcs: Vec<Htlc>,
  // csv_delay: number;
  // private: number;
  // initiator: number;
  // chan_status_flags: string;
  // local_chan_reserve_sat: number;
  // remote_chan_reserve_sat: number;
  // static_remote_key: boolean;
  // commitment_type: number;
  // lifetime: number;
  // uptime: number;
  // close_address: string;
  // push_amount_sat: number;
  // thaw_height: number;
  const parsedChannels = channels.map((channel, index: number) => {
    return {
      remote_pubkey: pubkey,
      capacity: convertMillisatsToSats(channel.total_msat.msat),
      local_balance: convertMillisatsToSats(channel.spendable_msat.msat),
      remote_balance: convertMillisatsToSats(channel.receivable_msat.msat),
      channel_point: `${bufferToHexString(channel.funding_txid)}:${index}`,
      active: getChannelStatus(channel.status),
      chat_id: shortChanIDtoInt64(bufferToHexString(channel.channel_id)), //This currently returning an empty string
    };
  });
  return parsedChannels;
}

function shortChanIDtoInt64(cid: string): string {
  if (typeof cid !== "string") return "";
  if (!(cid.includes(":") || cid.includes("x"))) return "";
  let a: string[] = [];
  const seps = [":", "x"];
  for (const sep of seps) {
    if (cid.includes(sep)) a = cid.split(sep);
  }
  if (!a) return "";
  if (!Array.isArray(a)) return "";
  if (!(a.length === 3)) return "";

  const blockHeight = long.fromString(a[0], true).shiftLeft(40);
  const txIndex = long.fromString(a[1], true).shiftLeft(16);
  const txPosition = long.fromString(a[2], true);

  return blockHeight.or(txIndex).or(txPosition).toString();
}

// CHANNELD_NORMAL
function getChannelStatus(status) {
  const derivedStatus = {};
  for (let i = 0; i < status.length; i++) {
    derivedStatus[status[i].split(":")[0]] = true;
  }
  if (derivedStatus[GreenlightChannelState.CHANNELD_NORMAL]) {
    return true;
  } else {
    false;
  }
}
