export async function getTransactionStatus(txid) {
  const res = await fetch(
    `https://mempool.space/testnet/api/tx/${txid}/status`
  );
  const status = await res.json();
  return status;
}

export async function getBlockTip() {
  const res = await fetch(
    `https://mempool.space/testnet/api/blocks/tip/height`
  );
  const status = await res.json();
  return status;
}
