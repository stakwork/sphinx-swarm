import * as BTC from "../api/btc";

type BitcoinNetwork = "bitcoin" | "regtest";

function mempoolApiBase(network: BitcoinNetwork) {
  return network === "bitcoin"
    ? "https://mempool.space/api"
    : "https://mempool.space/testnet/api";
}

export async function getTransactionStatus(
  txid: string,
  network: BitcoinNetwork,
  btcTag: string
) {
  if (network === "regtest") {
    return await BTC.get_transaction_status(btcTag, txid);
  }

  const res = await fetch(
    `${mempoolApiBase(network)}/tx/${txid}/status`
  );
  const status = await res.json();
  return status;
}

export async function getBlockTip(network: BitcoinNetwork, btcTag: string) {
  if (network === "regtest") {
    const info = await BTC.get_info(btcTag);
    return info.blocks;
  }

  const res = await fetch(`${mempoolApiBase(network)}/blocks/tip/height`);
  const status = await res.json();
  return status;
}
