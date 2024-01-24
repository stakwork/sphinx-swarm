<script>
  import {
    Button,
    Tabs,
    Tab,
    TabContent,
    Toggle,
    InlineNotification,
  } from "carbon-components-svelte";
  import { onMount } from "svelte";
  import SetupAdmin from "./components/NavFiberAdmin.svelte";
  import EnpointPermission from "./components/EnpointPermission.svelte";
  import {
    update_graph_accessibility,
    get_graph_accessibility,
  } from "./api/swarm";

  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8001";

  let toggled = false;
  $: disabled = false;
  $: show_notification = false;
  $: message = "";
  $: success = false;
  $: firstTime = false;

  async function toggleGraphStatus(state) {
    if (firstTime) {
      return;
    }
    disabled = true;
    const result = await update_graph_accessibility(state.toggled);
    const parsedResult = JSON.parse(result);
    message = parsedResult.message;
    success = parsedResult.success;
    show_notification = true;
    disabled = false;
  }

  onMount(async () => {
    //get graph state
    const result = await get_graph_accessibility();
    const parsedResult = JSON.parse(result);
    if (parsedResult.success) {
      firstTime = true;
      toggled = parsedResult.data.isPublic;
    }
    setTimeout(() => {
      firstTime = false;
    }, 500);
  });
</script>

<div class="nav-wrapper">
  {#if show_notification}
    <InlineNotification
      lowContrast
      kind={success ? "success" : "error"}
      title={success ? "Success:" : "Error:"}
      subtitle={message}
      timeout={3000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  <div class="heading_container">
    <Button target="_blank" href={link}>Open Second Brain</Button>
    <div>
      <Toggle
        labelText="Toggle Graph Accesibility"
        labelA="Private"
        labelB="Public"
        bind:toggled
        on:toggle={(e) => toggleGraphStatus(e.detail)}
        {disabled}
      />
    </div>
  </div>
  <div class="tab-container">
    <Tabs>
      <Tab label="Setup Admin"></Tab>
      <Tab label="Endpoint Permissions"></Tab>
      <svelte:fragment slot="content">
        <TabContent>
          <SetupAdmin></SetupAdmin>
        </TabContent>
        <TabContent>
          <EnpointPermission />
        </TabContent>
      </svelte:fragment>
    </Tabs>
  </div>
</div>

<style>
  .nav-wrapper {
    padding: 0px 25px;
  }

  .tab-container {
    margin-top: 1.5rem;
    margin-bottom: 1.5rem;
    width: 100%;
  }

  .heading_container {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
  }
</style>
