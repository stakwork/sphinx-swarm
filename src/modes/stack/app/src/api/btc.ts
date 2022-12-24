import { send_cmd, Cmd } from "./cmd";

export interface BtcInfo {
  automatic_pruning: boolean | any;
  bestblockhash: string;
  blocks: number;
  chain: string;
  chainwork: string;
  difficulty: number;
  headers: number;
  initialblockdownload: boolean;
  mediantime: number;
  prune_target_size: number;
  pruned: boolean;
  pruneheight: number | any;
  size_on_disk: number;
  verificationprogress: number;
  warnings: string;
}

async function btcCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Bitcoind", { cmd, content }, tag);
}

export async function get_info(tag: string) {
  return await btcCmd("GetInfo", tag);
}

export async function test_mine(tag: string, blocks: number, address: string) {
  return await btcCmd("TestMine", tag, { blocks, address });
}
