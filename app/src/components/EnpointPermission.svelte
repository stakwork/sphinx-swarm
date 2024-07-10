<script>
  import { onMount } from "svelte";
  import { list_all_paid_endpoint } from "../api/swarm";
  import Endpoint from "./Endpoint.svelte";

  $: allEndpoints = [];
  $: success = false;

  async function getAllPaidEndpoint() {
    const endpoints = await list_all_paid_endpoint();
    const parsedEndpoints = JSON.parse(endpoints);
    const endpointsObj = {};
    const tempAllEndpoints = [];
    if (parsedEndpoints.success) {
      for (let i = 0; i < parsedEndpoints.endpoints.length; i++) {
        const parsedEndpoint = parsedEndpoints.endpoints[i];
        const routeDescription = parsedEndpoint.route_description;
        if (endpointsObj[routeDescription]) {
          endpointsObj[routeDescription].id.push(parsedEndpoint.id);
        } else {
          endpointsObj[routeDescription] = {
            id: [parsedEndpoint.id],
            route_description: routeDescription,
            price: parsedEndpoint.price,
            endpoint: parsedEndpoint.endpoint,
            status: parsedEndpoint.status,
          };
        }
      }
      for (let key in endpointsObj) {
        tempAllEndpoints.push(endpointsObj[key]);
      }
      allEndpoints = [...tempAllEndpoints];
    }
  }

  function handleCustomEvent(event) {
    success = event.detail;

    setTimeout(() => {
      success = false;
    }, 5000);
  }

  onMount(async () => {
    await getAllPaidEndpoint();
  });
</script>

<div class="endpoint-permission-container">
  <div class="endpoint-header-container">
    <h2 class="endpoint_header">Payments</h2>
    {#if success}
      <div class="success_container">
        <img src="swarm/check_circle.svg" alt="success" />
        <p class="success_text">Endpoint Updated</p>
      </div>
    {/if}
  </div>
  {#each allEndpoints as endpoint, index (endpoint.route_description)}
    <Endpoint
      on:customEvent={handleCustomEvent}
      description={endpoint.route_description}
      toggled={endpoint.status}
      id={endpoint.id}
    />
  {/each}
</div>

<style>
  .endpoint-permission-container {
    padding-top: 1.5rem;
    padding-left: 2.25rem;
    padding-right: 2.25rem;
  }

  .endpoint-header-container {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.2rem 0rem;
  }

  .endpoint_header {
    font-family: "Barlow";
    font-size: 1.125rem;
    font-weight: 700;
    line-height: 1rem;
    letter-spacing: 0.01em;
  }

  .success_container {
    display: flex;
    align-items: center;
  }

  .success_text {
    font-family: "Roboto";
    font-size: 0.8125rem;
    font-weight: 400;
    line-height: 1rem;
    letter-spacing: 0.01em;
    color: #49c998;
    margin-left: 0.75rem;
  }
</style>
