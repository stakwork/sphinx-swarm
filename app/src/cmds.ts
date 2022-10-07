export const cmds = [
  "$CLN getinfo",
  "$CLN newaddr",
  "export PUBKEY=***",
  "$CLN connect $PUBKEY $HOST 9738",
  "$CLN fundchannel $CHAN_ID 100000",
  "$CLN keysend $PUBKEY 1000",
  "$CLN listfunds",
];
