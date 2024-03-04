<script>
  import {
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
  import { stack } from "./store";

  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8001";
  if ($stack && $stack.custom_2b_domain) {
    link = `https://${$stack.custom_2b_domain}`;
  }

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
    <div class="title_container">
      <img
        src="swarm/second-brain.svg"
        alt="Second Brain Logo"
        class="logo_image"
      />
      <p class="title">Second Brain</p>
    </div>

    <a target="_blank" rel="noreferrer" href={link}>
      <div class="open_link">
        <img src="swarm/link.svg" alt="link" class="link_image" />
        <p class="link_text">Open</p>
      </div></a
    >
  </div>
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
  .tab-container {
    margin-top: 1.5rem;
    margin-bottom: 1.5rem;
    width: 100%;
  }

  .heading_container {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: #1c1e26;
    padding: 1.5rem 2.25rem 1.75rem 2.25rem;
  }

  .title_container {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .title {
    color: white;
    text-transform: capitalize;
    font-family: "Barlow";
    font-size: 1.375rem;
    font-weight: 700;
    line-height: 0.875rem; /* 63.636% */
  }

  .open_link {
    display: flex;
    align-items: center;
    height: 2rem;
    padding: 0.75rem 0.75rem 0.75rem 0.5rem;
    gap: 0.5rem;
    border-radius: 0.375rem;
    background: #303342;
  }

  a {
    text-decoration: none;
  }

  .link_text {
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 500;
    line-height: 1.1875rem;
  }

  .link_image {
    width: 1.25rem;
    height: 1.25rem;
  }
</style>
