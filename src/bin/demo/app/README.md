docker exec -it cln_1.sphinx sh

lightning-cli --network=regtest getinfo

lightning-cli --network=regtest newaddr

lightning-cli --network=regtest getroute 036bebdc8ad27b5d9bd14163e9fea5617ac8618838aa7c0cae19d43391a9feb9db 1000 10
