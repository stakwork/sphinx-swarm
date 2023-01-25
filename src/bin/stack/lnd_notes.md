### connect to lnd1

`docker exec -it lnd1.sphinx sh`

`lncli --network=regtest --lnddir=/home/.lnd getinfo`

`exit` to quit

### connect to lnd2

`docker exec -it lnd2.sphinx sh`

`lncli --network=regtest --lnddir=/home/.lnd --rpcserver=localhost:10010 getinfo`

### connect to bitcoin

`docker exec -it bitcoind.sphinx sh`

find the bitcoind password in vol/stack/config.json

`export BTC_PASS=hEuW98KoErni`

`bitcoin-cli -regtest -rpcuser=sphinx -rpcpassword=$BTC_PASS -getinfo`

generate an address in LND, then

`export ADDY=xxxxxxx`

`bitcoin-cli -regtest -rpcuser=sphinx -rpcpassword=$BTC_PASS generatetoaddress 6 "$ADDY"`

### other useful commands

`lncli newaddress p2wkh`

`lncli --network=regtest --lnddir=/home/.lnd connect $PUBKEY@$HOST:$PORT`

`lncli openchannel $PUBKEY --local_amt=1000000 --push_amt=500000 --sat_per_byte=6`
