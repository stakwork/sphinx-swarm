# Fluent Bit sidecar helpers.
#
# Config dry-run validated against fluent/fluent-bit:3.2.10.
#
# busted is not in the repo CI image by default. Install with:
#   apt-get update && apt-get install -y lua5.1 luarocks
#   luarocks install busted
# Alpine:
#   apk add lua5.1 lua5.1-dev luarocks5.1 gcc musl-dev wget unzip make
#   luarocks-5.1 install busted
.PHONY: test

test:
	./tests/run.sh
