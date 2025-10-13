<script lang="ts">
  import { InlineNotification } from "carbon-components-svelte";
  import NodeVersionupdater from "./components/NodeVersionupdater.svelte";
  import { selectedNode } from "./store";
  let success = false;
  let notification_message = "";
  let show_notification = false;

  export let updateBody = () => {};

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

<main class="repo2graph_container">
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
  .repo2graph_container {
    padding: 1rem;
  }
</style>
