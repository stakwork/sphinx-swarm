### test proxy

export TEST_PROXY=true

cargo run --bin cln

### make a proxy user

check in logs for proxy2 admin token

export ADMIN_TOKEN=ie0UZBS2Ht37

curl http://localhost:5555/list -H "x-admin-token: $ADMIN_TOKEN"

curl -X POST http://localhost:5555/generate?sats=100 -H "x-admin-token: $ADMIN_TOKEN"

```js
var lndpk =
  "037c452bcab9a82ac869ff198f81eda02d6ba3e4e9bbf5b3f092be524a1803db18";
var scid = "1099511758849";
window.route_hint = `${lndpk}:${scid}`;
```

03cd01230c28e29300f1a10892e3421d5474d32fccbd34d60f315c556c443b7e38

curl http://localhost:5555/balances -H "x-admin-token: $ADMIN_TOKEN"

### copy creds for proxy js test

docker cp proxy.sphinx:/app/proxy/macaroons/ ./src/bin/cln/creds/

docker cp proxy.sphinx:/app/proxy/tls.cert ./src/bin/cln/creds/tls.cert

### sphinx-proxy testjs

export NODE_ENV=swarmProxyTest

export ADMIN_TOKEN=X5Wh04zmPB0H

nvm use 16

yarn test

### lncli

lncli --network=regtest --lnddir=/home/.lnd --rpcserver=127.0.0.1:10011 getinfo

lncli --network=regtest --lnddir=/home/.lnd --rpcserver=127.0.0.1:10011 sendpayment 02c7046d20f62012362ccf835fe5b4d4a1708e518592f216afeefabeadfc20154b 500 --keysend
