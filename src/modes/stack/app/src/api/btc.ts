import { send_cmd, Cmd } from "./cmd";

async function btcCmd(cmd: Cmd, tag: string, content?: any) {
  return await send_cmd("Bitcoind", { cmd, content }, tag);
}

export async function get_info(tag: string) {
  return await btcCmd("GetInfo", tag);
}
