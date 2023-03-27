<script lang="ts">
  import { selectedNode, exitedNodes, nodes_exited } from "./store";
  import {
    Loading,
    OverflowMenu,
    OverflowMenuItem,
  } from "carbon-components-svelte";
  import Flow from "./Flow.svelte";
  import Controller from "./controls/Controller.svelte";
  import AddNode from "./nodes/AddNode.svelte";
  import NodeLogs from "./nodes/NodeLogs.svelte";
  import NodeAction from "./nodes/NodeAction.svelte";
  import NodeUpdate from "./nodes/NodeUpdate.svelte";
  import { stack, logoutUser, containers } from "./store";
  import { afterUpdate, onMount } from "svelte";
  import * as api from "./api";
  import type { Stack } from "./nodes";
  import User from "carbon-icons-svelte/lib/User.svelte";
  import ChangePassword from "./auth/ChangePassword.svelte";
  import type { Container } from "./api/swarm";

  let selectedName = "";

  async function getConfig() {
    const stackRemote: Stack = await api.swarm.get_config();
    if (stackRemote.nodes !== $stack.nodes) {
      stack.set(stackRemote);
    }
  }

  async function listContainers() {
    const res: Container[] = await api.swarm.list_containers();
    if (res) containers.set(res);
  }

  onMount(() => {
    listContainers();
    getConfig();
  });

  type DashboardPage = "main" | "change_password";
  let page: DashboardPage = "main";

  async function backToMain() {
    page = "main";
  }
  function toChangePassword() {
    page = "change_password";
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
        body.classList.add(`selected-${node}`);
        body.classList.add(`${node}-stopped`);
      });
    }
  }

  function addStopClass(event) {
    body.classList.add(`${event.detail.text}-stopped`);
  }

  function removeStopClass(event) {
    if (body.classList.contains(`${event.detail.text}-stopped`)) {
      body.classList.remove(`${event.detail.text}-stopped`);
    }
  }
</script>

<main>
  <header>
    <div class="lefty logo-wrap">
      <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
      <span class="stack-title">Sphinx Stack</span>
    </div>

    <section class="header-btn-wrap">
      {#if $selectedNode}
        <NodeUpdate />
      {/if}

      {#if $selectedNode && $selectedNode.place === "Internal"}
        <NodeLogs nodeName={$selectedNode.name} />

        <NodeAction
          on:stop_message={addStopClass}
          on:start_message={removeStopClass}
        />
      {/if}
    </section>

    <AddNode />
    <section class="menu-btn">
      <OverflowMenu icon={User} flipped>
        <OverflowMenuItem on:click={toChangePassword} text="Change Password" />
        <OverflowMenuItem on:click={logoutUser} text="Logout" />
      </OverflowMenu>
    </section>
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
          <Loading />
        </div>
      {/if}
      <Controller />
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
    background: #1a242e;
    align-items: center;
    border-bottom: 1px solid #101317;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
  }
  .logo-wrap {
    display: flex;
    align-items: center;
  }

  .logo-wrap .logo {
    width: 70px;
    padding: 12px;
    margin-left: 2.5rem;
  }
  .body {
    display: flex;
    height: 100%;
  }
  .lefty {
    width: 18rem;
    max-width: 18rem;
    height: 100%;
    border-right: 1px solid #101317;
  }
  .stack-title {
    color: white;
    margin-left: 0.5rem;
    font-size: 1.2rem;
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
</style>