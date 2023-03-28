import { bufferToHexString } from "./";

export function parseClnGetInfo(res) {
  const pubkey = bufferToHexString(res.id);
  return { identity_pubkey: pubkey };
}

export function parseClnListPeerRes(res: { peers: { id: Buffer; netaddr }[] }) {
  // pub_key: string;
  // address: string;
  // bytes_sent: number;
  // bytes_recv: number;
  // sat_sent: number;
  // sat_recv: number;
  // inbound: number;
  // ping_time: number;
  // sync_type: number;
  const peers = res.peers.map((peer) => {
    return {
      pub_key: bufferToHexString(peer.id),
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
  return peers;
}
