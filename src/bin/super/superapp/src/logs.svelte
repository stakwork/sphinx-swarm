<script lang="ts">
  import { onMount } from "svelte";
  import { get_super_admin_logs } from "../../../../../app/src/api/swarm";
  import { Loading, ToastNotification } from "carbon-components-svelte";
  import { cleanLog } from "./utils/logs";
  let error_message = "";
  let loading = true;
  let logs: string[] = [];

  onMount(async () => {
    // get logs
    logs = [];
    try {
      let res = await get_super_admin_logs();
      if (res.success && res.data) {
        logs = res.data.map((log: string) => cleanLog(log));
      } else {
        logs[0] = res.message;
      }
    } catch (error) {
      console.log("error: ", error);
      logs[0] = "Error occured while trying to get lightning bots details";
    } finally {
      loading = false;
    }
  });
</script>

<main>
  <div>
    {#if loading}
      <Loading />
    {/if}
    {#if logs.length > 0}
      <div class="logs">
        {#each logs as log}
          <pre class="log">{log}</pre>
        {/each}
      </div>
    {:else}
      <div class="empty_state_container">
        <p>We are unable to fetch logs at the moment, please contact admin</p>
      </div>
    {/if}
  </div>
</main>

<style>
  .logs {
    background: #393939;
    width: 100%;
    min-height: 30vh;
    max-height: 90vh;
    overflow: auto;
    padding: 0.3rem 0.5rem;
    display: flex;
    flex-direction: column-reverse;
    background-color: #1e1e1e;
  }
  .log {
    font-family: monospace;
    font-size: 0.8rem;
    background-color: #1e1e1e;
    color: white;
    padding: 0.3rem 0.5rem;
    white-space: pre-wrap;
    margin-bottom: 0.4rem;
  }

  .empty_state_container p {
    padding: 2rem;
  }
</style>
