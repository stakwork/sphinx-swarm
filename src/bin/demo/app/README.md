docker exec -it cln_1.sphinx sh

lightning-cli --network=regtest getinfo

lightning-cli --network=regtest newaddr

lightning-cli --network=regtest getroute 030f5205642b40c64ac5c575f4f365ca90b692f13808b46d827fdb1b6026a3e6c2 1000 10

lightning-cli --network=regtest listpeerchannels
