<script lang="ts">
  import { Button, Modal } from "carbon-components-svelte";
  import Logs from "carbon-icons-svelte/lib/CloudLogging.svelte";
  import * as api from "./api";
  import { onDestroy } from "svelte";

  let open = false;
  export let nodeName = "";
  let logs = [];

  async function getNodeLogs() {
    open = true;
    const theLogs = await api.swarm.get_logs(`${nodeName}.sphinx`);
    logs = theLogs.reverse();
  }

  onDestroy(() => {
    logs = [];
  });
</script>

<section class="get-logs-btn">
  <Button type="button" size="field" icon={Logs} on:click={getNodeLogs}
    >Get Logs</Button
  >

  <Modal
    bind:open
    modalHeading={`${nodeName.toLocaleUpperCase()} Logs`}
    hasForm={true}
    class="get-logs-modal"
    passiveModal
    on:click:button--secondary={() => (open = !open)}
  >
    <section class="modal-content">
      <div class="logs">
        {#each logs as log}
          <div class="log">{log}</div>
        {/each}
      </div>
      <!-- <TextArea rows={15} value={String(logs)} /> -->
    </section>
  </Modal>
</section>

<style>
  .get-logs-btn {
    margin-left: 20px;
  }
  .modal-content {
    padding: 0px 1.5rem;
  }
  .logs {
    background: #393939;
    width: 100%;
    height: 100%;
    max-height: 400px;
    overflow: auto;
    padding: 0.3rem 0.5rem;
    display: flex;
    flex-direction: column-reverse;
  }
  .log {
    color: white;
    margin: 1px 0;
  }
</style>
