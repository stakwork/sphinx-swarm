<script>
  import { InlineNotification } from "carbon-components-svelte";
  import { onMount } from "svelte";
  import SetupAdmin from "./components/NavFiberAdmin.svelte";
  import EnpointPermission from "./components/EnpointPermission.svelte";
  import General from "./components/SecondBrain/general.svelte";
  import { get_graph_accessibility } from "./api/swarm";
  import { stack, selectedNode } from "./store";
  import Roles from "./components/SecondBrain/roles/roles.svelte";
  import Apikeys from "./components/SecondBrain/apikeys.svelte";

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
  $: currentTab = "General";

  const tabs = ["General", "Roles", "Payments", "Api Keys", "Test"];

  function setActiveTab(tab) {
    currentTab = tab;
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
      <div class="title_version_container">
        <p class="title">Second Brain</p>
        <p class="version">{$selectedNode.version}</p>
      </div>
    </div>

    <a target="_blank" rel="noreferrer" href={link}>
      <div class="open_link">
        <img src="swarm/link.svg" alt="link" class="link_image" />
        <p class="link_text">Open</p>
      </div></a
    >
  </div>
  <div class="tab-container">
    <div class="tab-header">
      {#each tabs as tab (tab)}
        <button
          class="tab_button"
          style={`${tab === currentTab ? "color: white; border-bottom: 0.125rem solid #618AFF;" : "color: #909BAA;"}`}
          on:click={() => setActiveTab(tab)}
        >
          {tab}
        </button>
      {/each}
    </div>
    <div class="tab-content">
      {#if currentTab === "General"}
        <General />
      {:else if currentTab === "Roles"}
        <!-- <SetupAdmin /> -->
        <Roles />
      {:else if currentTab === "Api Keys"}
        <Apikeys />
      {:else if currentTab === "Test"}
        <div>Testing new build Again and Again</div>
      {:else}
        <EnpointPermission />
      {/if}
    </div>
  </div>
</div>

<style>
  .tab-container {
    display: flex;
    flex-direction: column;
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

  .title_version_container {
    display: flex;
    flex-direction: column;
  }

  .title {
    color: white;
    text-transform: capitalize;
    font-family: "Barlow";
    font-size: 1.375rem;
    font-weight: 700;
    line-height: 0.875rem; /* 63.636% */
  }

  .version {
    font-family: "Barlow";
    font-size: 0.8rem;
    margin-top: 0.4rem;
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

  .tab-header {
    display: flex;
    align-items: flex-start;
    gap: 3.5rem;
    background-color: #1c1e26;
    padding-left: 2.25rem;
    padding-right: 2.25rem;
  }

  .tab_button {
    color: #fff;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 500;
    line-height: normal;
    padding-bottom: 0.75rem;
    padding-left: 0;
    padding-right: 0;
    cursor: pointer;
    text-transform: capitalize;
    background-color: transparent;
    border: none;
    outline: none;
  }

  .tab-content {
    display: flex;
    flex-direction: column;
  }
</style>
