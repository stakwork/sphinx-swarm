<script lang="ts">
  import {
    stack,
    logoutUser,
    containers,
    sleep,
    selectedNode,
    nodes_exited,
    current_swarm_user,
    swarmVersion,
  } from "./store";
  import {
    Loading,
    OverflowMenu,
    OverflowMenuItem,
    InlineLoading,
  } from "carbon-components-svelte";
  import Flow from "./Flow.svelte";
  import Controller from "./controls/Controller.svelte";
  import NodeLogs from "./nodes/NodeLogs.svelte";
  import NodeStats from "./nodes/NodeStats.svelte";
  import NodeAction from "./nodes/NodeAction.svelte";
  import NodeUpdate from "./nodes/NodeUpdate.svelte";
  import { onMount } from "svelte";
  import * as api from "./api";
  import type { Stack } from "./nodes";
  import User from "carbon-icons-svelte/lib/User.svelte";
  import ChangePassword from "./auth/ChangePassword.svelte";
  import { get_signedin_user_details, type Container } from "./api/swarm";
  import { getImageVersion } from "./helpers/swarm";
  import RestartNode from "./nodes/RestartNode.svelte";
  let selectedName = "";

  $: nodes = [...$stack.nodes];

  async function getNodeVersion() {
    await getImageVersion(stack, selectedNode);
    nodes = [...$stack.nodes];
    body = body;
  }

  async function pollConfig() {
    let ready = false;
    while (!ready) {
      const stackReady = await getConfig();
      if (stackReady) ready = true;
      await sleep(3000);
    }
  }

  async function getConfig(): Promise<boolean> {
    const stackRemote: Stack = await api.swarm.get_config();
    if (stackRemote.nodes !== $stack.nodes) {
      stack.set(stackRemote);
      // get node version
      getNodeVersion();
    }
    return stackRemote.ready;
  }

  async function handleSignedInUser() {
    try {
      const user = await get_signedin_user_details();
      if (typeof user === "object") {
        current_swarm_user.set(user);
      }
    } catch (error) {
      console.log(`Error getting user: ${error}`);
    }
  }

  async function listContainers() {
    const res: Container[] = await api.swarm.list_containers();
    if (res) containers.set(res);
  }

  onMount(() => {
    // get user details
    handleSignedInUser();
    listContainers();
    pollConfig();
  });

  type DashboardPage = "main" | "change_password";
  let page: DashboardPage = "main";

  async function backToMain() {
    page = "main";
  }
  function toChangePassword() {
    page = "change_password";
  }

  async function updateSwarm() {
    await api.swarm.update_swarm();
  }

  function updateBody() {
    body = body;
  }

  let body;

  $: {
    if (body) {
      if ($selectedNode) {
        // Remove the previous name saved in state
        body.classList.remove(`selected-${selectedName}`);
        // add the new classname
        body.classList.add(`selected-${$selectedNode.name}`);
        // save name to state
        selectedName = $selectedNode.name;
      } else {
        body.classList.remove(`selected-${selectedName}`);
      }
    }
  }
  $: {
    if ($nodes_exited) {
      $nodes_exited.forEach((node) => {
        body?.classList.add(`selected-${node}`);
        body?.classList.add(`${node}-stopped`);
      });
    }
  }

  function addStopClass(event) {
    body?.classList.add(`${event.detail.text}-stopped`);
  }

  function removeStopClass(event) {
    if (body?.classList.contains(`${event.detail.text}-stopped`)) {
      body?.classList.remove(`${event.detail.text}-stopped`);
    }
  }
</script>

<main>
  <header>
    <div class="head_section">
      <div class="lefty logo-wrap">
        <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
        <div>
          <span
            class="stack-title"
            style={`color:${$stack.ready ? "white" : "#999"}`}
            >Sphinx Stack
          </span>
          <p class="swarm_version">{$swarmVersion}</p>
        </div>
        {#if !$stack.ready}
          <InlineLoading />
        {/if}
      </div>

      <section class="header-btn-wrap">
        {#if $selectedNode && $selectedNode.place === "Internal"}
          <NodeLogs nodeName={$selectedNode.name} />

          <NodeAction
            on:stop_message={addStopClass}
            on:start_message={removeStopClass}
          />

          <NodeUpdate {updateBody} />

          <RestartNode />
        {/if}
      </section>
    </div>
    <div class="head_section">
      {#if $stack.ready}
        <!-- <Onboarding /> -->
        <NodeStats />
      {/if}
      <!-- <AddNode /> -->
      <section class="menu-btn">
        <OverflowMenu icon={User} flipped>
          <OverflowMenuItem on:click={updateSwarm} text="Update" />
          <OverflowMenuItem
            on:click={toChangePassword}
            text="Change Password"
          />
          <OverflowMenuItem on:click={logoutUser} text="Logout" />
        </OverflowMenu>
      </section>
    </div>
  </header>
  <div class="body" bind:this={body}>
    {#if page === "change_password"}
      <ChangePassword back={backToMain} />
    {:else if page === "main"}
      {#if $stack.nodes.length}
        {#key body}
          <Flow />
        {/key}
      {:else}
        <div class="loader">
          letting
          <Loading />
        </div>
      {/if}
      <Controller {updateBody} />
    {/if}
  </div>
</main>

<style>
  main {
    height: 100vh;
    width: 100vw;
    display: flex;
    flex-direction: column;
  }
  header {
    height: 4.2rem;
    min-height: 4.2rem;
    display: flex;
    background: #23252f;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid rgba(0, 0, 0, 0.3);
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
  }
  .logo-wrap {
    display: flex;
    align-items: center;
    gap: 2.5rem;
  }

  .head_section {
    display: flex;
    align-items: center;
  }

  .logo-wrap .logo {
    width: 70px;
    padding: 12px;
  }
  .body {
    display: flex;
    height: 100%;
  }
  .lefty {
    height: 100%;
    border-right: 1px solid #101317;
    padding: 1rem;
  }
  .stack-title {
    color: white;
    font-size: 1.2rem;
    width: 18rem;
  }
  .header-btn-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
  }
  .loader {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    align-items: center;
    justify-content: center;
    justify-items: center;
  }
  .menu-btn {
    background-color: transparent;
    color: #fefefe;
    border: 0;
    outline: 0;
    margin-right: 2.5rem;
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
  }

  .swarm_version {
    font-size: 0.9rem;
    font-family: "Barlow";
  }
</style>
