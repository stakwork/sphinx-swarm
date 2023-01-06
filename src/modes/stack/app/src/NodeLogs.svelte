<script lang="ts">
  import { Button, Modal, TextArea } from "carbon-components-svelte";
  import Logs from "carbon-icons-svelte/lib/CloudLogging.svelte";
  import * as api from "./api";

  let open = false;
  export let nodeName = "";
  let logs = "";

  async function getNodeLogs() {
    open = true;
    logs = await api.swarm.get_logs(`${nodeName}.sphinx`);
  }
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
      <TextArea rows={15} value={String(logs)} />
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
</style>
