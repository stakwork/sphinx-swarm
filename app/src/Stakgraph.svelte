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

<main class="stakgraph_container">
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
  <NodeVersionupdater
    handleSuccess={handleNodeVersionUpdateSuccess}
    handleError={handleNodeVersionUpdateError}
  />
</main>

<style>
  .stakgraph_container {
    padding: 1rem;
  }
</style>
