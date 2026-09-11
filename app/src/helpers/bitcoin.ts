import * as btcApi from "../api/btc";
import type { BtcTransactionStatus } from "../api/btc";

interface ChainLookupOptions {
  network?: string;
  bitcoindTag?: string;
}

function getMempoolBaseUrl(network?: string) {
  if (network === "bitcoin") {
    return "https://mempool.space/api";
  }
  return "https://mempool.space/testnet/api";
}

export async function getTransactionStatus(
  txid: string,
  options: ChainLookupOptions = {}
): Promise<BtcTransactionStatus> {
  if (options.network === "regtest") {
    if (!options.bitcoindTag) {
      return { confirmed: false, block_height: null };
    }
    return await btcApi.get_transaction_status(options.bitcoindTag, txid);
  }

  const res = await fetch(`${getMempoolBaseUrl(options.network)}/tx/${txid}/status`);
  const status = await res.json();
  return status;
}

export async function getBlockTip(options: ChainLookupOptions = {}) {
  if (options.network === "regtest") {
    if (!options.bitcoindTag) {
      return 0;
    }
    const info = await btcApi.get_info(options.bitcoindTag);
    return info?.blocks || 0;
  }

  const res = await fetch(`${getMempoolBaseUrl(options.network)}/blocks/tip/height`);
  const status = await res.json();
  return status;
}
