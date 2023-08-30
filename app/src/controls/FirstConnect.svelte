<script lang="ts">
  import { stack } from "../store";
  import type { Stack, Node } from "../nodes";
  import QrCode from "svelte-qrcode";

  $: cln_node = $stack && $stack.nodes.find((n) => n.type === "Cln");

  function makeMqttHost(s: Stack, n: Node) {
    if (s.ip) {
      return `${s.ip}:1883`;
    } else if (s.host && n) {
      return `mqtt-${n.name}.${s.host}:8883`;
    } else {
      return `127.0.0.1:1883`;
    }
  }
  $: mqttHost = makeMqttHost($stack, cln_node);

  $: relay_node = $stack && $stack.nodes.find((n) => n.type === "Relay");
  function makeRelayHost(s: Stack, n: Node) {
    if (s.host && n) {
      return `${n.name}.${s.host}`;
    } else {
      return `127.0.0.1:3000`;
    }
  }
  $: relayHost = makeRelayHost($stack, relay_node);

  function makeQR(mqtt: string, network: string) {
    return `sphinx.chat://?action=glyph&mqtt=${mqtt}&network=${network}&relay=${relayHost}`;
  }

  function copyQR() {
    const qr = makeQR(mqttHost, $stack.network);
    navigator.clipboard.writeText(qr);
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
        <span>{mqttHost}</span>
        <span>{$stack.network}</span>
      </div>
    </div>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div class="qr-wrap" on:click={copyQR}>
      <QrCode size={256} padding={4} value={makeQR(mqttHost, $stack.network)} />
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
