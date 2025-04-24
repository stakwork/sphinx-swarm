import {
  bufferToHexString,
  convertMillisatsToSats,
  parseDate,
  shortTransactionId,
} from "./";
import long from "long";
import type { LndChannel, LndPeer } from "../api/lnd";
import type { LightningPeer } from "../nodes";
import { list_peers } from "../api/cln";
import { peers } from "../store";

enum ClnChannelState {
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

export function parseClnListPeerChannelsRes(res: {
  channels: any[];
}): LndChannel[] {
  return parseClnPeerChannelList(res.channels);
}

export function parseClnListPeerRes(res: {
  peers: { id: Buffer; connected: boolean; netaddr; channels }[];
}): LndPeer[] {
  if (typeof res !== "object") {
    return [];
  }
  // pub_key: string;
  // address: string;
  // bytes_sent: number;
  // bytes_recv: number;
  // sat_sent: number;
  // sat_recv: number;
  // inbound: number;
  // ping_time: number;
  // sync_type: number;
  // let channels: LndChannel[] = [];
  const peers = res.peers.map((peer) => {
    const pub_key = bufferToHexString(peer.id);
    // channels = [...channels, ...parseClnChannelList(peer.channels, pub_key)];
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
      connected: peer.connected,
    };
  });
  return peers;
}

function parseClnPeerChannelList(channels: any): LndChannel[] {
  if (!channels) {
    return [];
  }
  if (!Array.isArray(channels)) {
    return [];
  }
  const parsedChannels = channels.map((channel, index: number) => {
    return <LndChannel>{
      remote_pubkey: bufferToHexString(channel.peer_id),
      capacity: convertMillisatsToSats(channel.total_msat.msat),
      local_balance: convertMillisatsToSats(channel.spendable_msat.msat),
      remote_balance: convertMillisatsToSats(channel.receivable_msat.msat),
      channel_point: `${bufferToHexString(channel.funding_txid)}:${index}`,
      active: getChannelStatus(channel.status),
      chan_id: shortChanIDtoInt64(channel.short_channel_id), //This currently returning an empty string
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
  if (derivedStatus[ClnChannelState.CHANNELD_NORMAL]) {
    return true;
  } else {
    return false;
  }
}

function convertPeerChannelArrayToObj(peerChanObj) {
  console.log("=>", peerChanObj);

  const obj = {};

  if (typeof peerChanObj !== "object") {
    return obj;
  }

  if (!peerChanObj) {
    return obj;
  }

  for (let i = 0; i < peerChanObj.channels.length; i++) {
    const channel = peerChanObj.channels[i];
    obj[channel.short_channel_id] = channel;
  }
  return obj;
}

export function parseClnListFunds(res): number {
  let balance = 0;
  if (typeof res !== "object") {
    return 0;
  }

  for (let i = 0; i < res.outputs.length; i++) {
    let output = res.outputs[i];
    if (
      output.status === 1 &&
      !output.reserved &&
      ouput.status == "confirmed"
    ) {
      balance += output.amount_msat.msat;
    }
  }
  return convertMillisatsToSats(balance);
}

export function parseUnconfirmedClnBalance(res): number {
  if (typeof res !== "object") {
    return 0;
  }
  let balance = 0;
  for (let i = 0; i < res.outputs.length; i++) {
    let output = res.outputs[i];
    if (output.status === 0 && !output.reserved) {
      balance += output.amount_msat.msat;
    }
  }
  return convertMillisatsToSats(balance);
}

export function parseClnPayments(transactions) {
  if (transactions.length > 0) {
    let trans = [];
    for (let i = 0; i < transactions.length; i++) {
      const transaction = transactions[i];
      const id =
        transaction.bolt11 || bufferToHexString(transaction.payment_hash);
      if (transaction.status === 2) {
        trans.push({
          id,
          index: `${i + 1}.`,
          invoice: shortTransactionId(id),
          date: parseDate(transaction.created_at),
          amount: `${convertMillisatsToSats(
            transaction.amount_sent_msat.msat
          ).toLocaleString()} sats`,
        });
      }
    }
    return trans;
  } else {
    return [];
  }
}

export function parseClnInvoices(transactions) {
  if (transactions.length > 0) {
    let trans = [];
    for (let i = 0; i < transactions.length; i++) {
      const transaction = transactions[i];
      const id = transaction.bolt11;
      if (transaction.status === 1) {
        trans.push({
          id,
          index: `${i + 1}.`,
          invoice: shortTransactionId(id),
          date: parseDate(transaction.paid_at),
          amount: `${convertMillisatsToSats(
            transaction.amount_received_msat?.msat
          ).toLocaleString()} sats`,
        });
      }
    }
    return trans;
  } else {
    return [];
  }
}

export function convertLightningPeersToObject(lightningPeers: LightningPeer[]) {
  const peersObj = {};
  for (let i = 0; i < lightningPeers.length; i++) {
    peersObj[lightningPeers[i].pubkey] = lightningPeers[i].alias;
  }
  return peersObj;
}

export function convertPeersToConnectObj(peers: LndPeer[]) {
  const connectPeerObj = {};
  if (!Array.isArray(peers)) {
    return connectPeerObj;
  }
  for (let i = 0; i < peers.length; i++) {
    const peer = peers[i];
    connectPeerObj[peer.pub_key] = peer.connected;
  }
  return connectPeerObj;
}

export async function fetchAndUpdateClnPeerStore(tag: string) {
  const peersData = await list_peers(tag);
  const thepeers = await parseClnListPeerRes(peersData);
  peers.update((peer) => {
    return { ...peer, [tag]: thepeers };
  });
}
