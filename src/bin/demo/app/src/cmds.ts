export const cmds = [
  "$CLN getinfo",
  "$CLN newaddr",
  "export PUBKEY=***",
  "$CLN connect $PUBKEY $HOST 9738",
  "$CLN fundchannel $PUBKEY 100000",
  "$CLN keysend $PUBKEY 10000",
  "$CLN listfunds",
];
