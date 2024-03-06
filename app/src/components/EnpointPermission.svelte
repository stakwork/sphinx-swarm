<script>
  import { onMount } from "svelte";
  import { list_all_paid_endpoint } from "../api/swarm";
  import Endpoint from "./Endpoint.svelte";

  $: allEndpoints = [];

  async function getAllPaidEndpoint() {
    const endpoints = await list_all_paid_endpoint();
    const parsedEndpoints = JSON.parse(endpoints);
    if (parsedEndpoints.success) {
      allEndpoints = [...parsedEndpoints.endpoints];
    }
  }

  onMount(async () => {
    await getAllPaidEndpoint();
  });
</script>

<div class="endpoint-permission-container">
  {#each allEndpoints as endpoint, index (endpoint.id)}
    <Endpoint
      description={endpoint.route_description}
      toggled={endpoint.status}
      id={endpoint.id}
    />
  {/each}
</div>

<style>
  .endpoint-permission-container {
    padding-top: 1.5rem;
  }
</style>
