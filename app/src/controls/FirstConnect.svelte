<script lang="ts">
  import { stack } from "../store";
  import QrCode from "svelte-qrcode";

  $: cln_node = $stack && $stack.nodes.find((n) => n.type === "Cln");
  $: host =
    $stack.host && cln_node
      ? `mqtt-${cln_node.name}.${$stack.host}:8883`
      : `127.0.0.1:1883`;

  function makeQR(mqtt: string, network: string) {
    return `sphinx.chat://?action=glyph&mqtt=${mqtt}&network=${network}`;
  }
</script>

<div class="wrap">
  <div class="head">Connect your Signer:</div>
  <div class="body">
    <div class="labels">
      <div class="label-section">
        <span>MQTT URL:</span>
        <span>Network:</span>
      </div>
      <div class="label-section">
        <span>{host}</span>
        <span>{$stack.network}</span>
      </div>
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
  .labels {
    display: flex;
    width: 100%;
    margin-bottom: 1rem;
  }
  .label-section {
    color: #ccc;
    font-weight: bold;
    font-size: 0.8rem;
    display: flex;
    flex-direction: column;
  }
  .label-section:first-child {
    width: 5.5rem;
  }
  .label-section:last-child {
    color: white;
    font-size: 0.8rem;
  }
  .label-section span:first-child {
    margin-bottom: 1rem;
  }
  .qr-wrap {
    margin-top: 1rem;
    height: 16rem;
    width: 16rem;
  }
</style>
