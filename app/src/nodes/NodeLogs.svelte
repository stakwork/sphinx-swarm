<script lang="ts">
  import { Button } from "carbon-components-svelte";
  import Logs from "carbon-icons-svelte/lib/CloudLogging.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import * as api from "../api";
  import { onDestroy } from "svelte";

  let open = false;
  export let nodeName = "";
  let logs = "";

  function cleanLog(log) {
    return log.replace(/\x1B\[[0-9;]*m/g, ""); // Remove ANSI escape codes
  }

  async function getNodeLogs() {
    open = true;
    const theLogs = await api.swarm.get_logs(`${nodeName}.sphinx`);
    if (theLogs) {
      logs = "";
      for (let i = 0; i < theLogs.length; i++) {
        logs = `${logs}${cleanLog(theLogs[i])}`;
      }
    }
  }

  onDestroy(() => {
    logs = "";
  });
</script>

<section class="get-logs-btn">
  <Button type="button" size="field" icon={Logs} on:click={getNodeLogs}
    >Get Logs</Button
  >

  <div class="modal" style={`display: ${open ? "block" : "none"}`}>
    <section class="modal-head">
      <button on:click={() => (open = !open)}>
        <ArrowLeft size={32} />
      </button>
      <h4 class="modal-title">{nodeName.toLocaleUpperCase()} Logs</h4>
    </section>
    <section class="modal-content">
      <div class="logs">
        <pre class="log">{logs}</pre>
      </div>
    </section>
  </div>
</section>

<style>
  .get-logs-btn {
    margin-left: 20px;
  }
  .modal {
    height: 88vh;
    z-index: 100;
    width: 98vw;
    position: absolute;
    left: 1%;
    right: 1%;
    bottom: 2%;
    background: #1a242e;
    border: 0.8px solid white;
  }
  .modal-head {
    display: flex;
    align-items: center;
    padding-top: 1rem;
    padding-left: 2.5rem;
  }
  .modal-head button {
    padding: 0;
    background: 0;
    border: 0;
    cursor: pointer;
    color: #fff;
    font-weight: 900;
  }
  .modal-head .modal-title {
    padding: 0;
    margin: 0;
    margin-left: 20px;
    font-size: 1.2rem;
    font-weight: 600;
  }
  .modal-content {
    padding: 2rem 2.5rem;
    padding-top: 1.2rem;
  }
  .logs {
    background: #393939;
    width: 100%;
    min-height: 30vh;
    max-height: 76vh;
    overflow: auto;
    padding: 0.3rem 0.5rem;
    display: flex;
    flex-direction: column-reverse;
  }
  .log {
    font-family: monospace;
    font-size: 0.8rem;
    background-color: #1e1e1e;
    color: white;
    padding: 1rem;
    white-space: pre-wrap;
  }
</style>
