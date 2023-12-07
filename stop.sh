if  [ $1 == "btc" ] || \
    [ $1 == "lnd" ] || \
    [ $1 == "cln" ] || \
    [ $1 == "proxy" ] || \
    [ $1 == "relay" ] || \
    [ $1 == "jarvis" ] || \
    [ $1 == "boltwall" ] || \
    [ $1 == "neo4j" ] || \
    [ $1 == "elastic" ] || \
    [ $1 == "navfiber" ] || \
    [ $1 == "cache" ] || \
    [ $1 == "lss" ] || \
    [ $1 == "mixer" ] || \
    [ $1 == "broker" ] || \
    [ $1 == "traefik" ] || \
    [ $1 == "swarm" ]
then
    echo "=> stop $1.sphinx"
    docker stop $1.sphinx
    docker rm $1.sphinx
else
    echo "=> invalid image name!"
fi

