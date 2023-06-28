<script lang="ts">
  import { stack } from "../store";
  import QrCode from "svelte-qrcode";

  $: cln_node = $stack && $stack.nodes.find((n) => n.type === "Cln");
  $: host =
    $stack.host && cln_node
      ? `${cln_node.name}-mqtt.${$stack.host}:8883`
      : `127.0.0.1:1883`;

  function makeQR(mqtt: string, network: string) {
    return `sphinx.chat://?action=glyph&mqtt=${mqtt}&network=${network}`;
  }
</script>

<div class="wrap">
  <div class="head">Connect your Signer:</div>
  <div class="body">
    <div class="mqtt-url">
      <span>MQTT URL:</span>
      <span>{host}</span>
    </div>
    <div class="qr-wrap">
      <QrCode size={256} padding={4} value={makeQR(host, $stack.network)} />
    </div>
  </div>
</div>

<style>
  .wrap {
    width: 100%;
    padding: 2rem;
  }
  .head {
    height: 4rem;
    display: flex;
    align-items: center;
  }
  .body {
    display: flex;
    flex-direction: column;
  }
  .mqtt-url {
    color: #ccc;
    font-weight: bold;
    font-size: 0.75rem;
    width: 100%;
    display: flex;
    align-items: center;
  }
  .mqtt-url span:first-child {
    margin-right: 1rem;
  }
  .qr-wrap {
    margin-top: 1rem;
    height: 16rem;
    width: 16rem;
  }
</style>
