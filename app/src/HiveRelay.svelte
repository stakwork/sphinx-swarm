<script lang="ts">
  import { InlineNotification } from "carbon-components-svelte";
  import NodeVersionupdater from "./components/NodeVersionupdater.svelte";
  export let updateBody = () => {};
  import { selectedNode } from "./store";
  let success = false;
  let notification_message = "";
  let show_notification = false;

  function handleNodeVersionUpdateSuccess() {
    updateBody();

    success = true;
    notification_message = `${$selectedNode.name} version updated successfully`;
    show_notification = true;
  }

  function handleNodeVersionUpdateError(errMsg: string) {
    success = false;
    notification_message = errMsg;
    show_notification = true;
  }
</script>

<main class="hiverelay_container">
  {#if show_notification}
    <InlineNotification
      lowContrast
      kind={success ? "success" : "error"}
      title={success ? "Success:" : "Error:"}
      subtitle={notification_message}
      timeout={9000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  {#if $selectedNode && $selectedNode.host}
    <div class="open-link">
      <a href={`https://${$selectedNode.host}`} target="_blank" rel="noreferrer">
        Open
      </a>
    </div>
  {/if}
  <NodeVersionupdater
    handleSuccess={handleNodeVersionUpdateSuccess}
    handleError={handleNodeVersionUpdateError}
  />
</main>

<style>
  .hiverelay_container {
    padding: 1rem;
  }
  .open-link {
    margin-bottom: 1rem;
  }
  .open-link a {
    color: #61dafb;
    text-decoration: none;
    font-size: 0.9rem;
  }
  .open-link a:hover {
    text-decoration: underline;
  }
</style>
