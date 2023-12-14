<script lang="ts">
  import { Dropdown, Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { onDestroy, onMount } from "svelte";
  import * as api from "../api";
  import {
    users,
    node_host,
    finishedOnboarding,
    adminIsCreatedForOnboarding,
    isOnboarding,
  } from "../store";
  import QrCode from "svelte-qrcode";
  import CopyIcon from "carbon-icons-svelte/lib/Copy.svelte";
  import { Chat } from "carbon-icons-svelte";

  $: adminUnconnected = $users?.find((u) => u.is_admin && !u.alias);

  export let tag = "";

  let myChats: api.tribes.TribeData[] = [];

  let fetchUsersInterval;

  onMount(async () => {
    if (!tag) return;
    const chats = await refreshTribes();
    getUsers();
  });

  onDestroy(() => {
    if (fetchUsersInterval) clearInterval(fetchUsersInterval);
  });

  $: items = myChats
    .filter((t) => !t.default_join)
    .map((t) => ({
      id: t.id,
      text: t.name,
    }));

  $: defaultTribes = myChats.filter((t) => t.default_join);

  let selectedTribe = "";

  async function refreshTribes() {
    const chats = await api.relay.get_chats(tag);
    if (Array.isArray(chats)) {
      myChats = chats;
    }
    return chats;
  }

  async function deleteTribe(id) {
    await api.relay.remove_default_tribe(tag, id);
    refreshTribes();
  }

  async function addDefaultTribe(id) {
    await api.relay.add_default_tribe(tag, id);
    refreshTribes();
    selectedTribe = "";
  }

  async function getUsers() {
    const userList = await api.relay.list_users(tag);
    const theAdmin = userList.users?.find((u) => u.is_admin);
    if (theAdmin?.public_key) {
      admin_pubkey = theAdmin.public_key;
    }
    users.set(userList.users);
    adminIsCreatedForOnboarding.set(true);
  }

  let showQr = false;

  let admin_pubkey = "";
  async function toggleQr() {
    showQr = !showQr;
  }

  function copyToClipboard(value) {
    navigator.clipboard.writeText(value);
  }

  $: qrString = `connect::${$node_host}::${admin_pubkey}`;
  $: $finishedOnboarding, determineToShowQr();
  function determineToShowQr() {
    if (
      $finishedOnboarding.hasChannels &&
      !$finishedOnboarding.hasAdmin &&
      $isOnboarding
    ) {
      showQr = false;
      toggleQr();
    }
  }
</script>

<div class="tribes-wrap">
  {#if adminUnconnected}
    <section class="admin-qr-wrap">
      <h1 class="admin-qr-label">Connection QR</h1>
      <div class="relay-admin-qr-btns">
        {#if showQr}
          <Button
            kind="tertiary"
            size="small"
            icon={CopyIcon}
            on:click={() => copyToClipboard(qrString)}>Copy</Button
          >
        {/if}
        <div class="btn-spacer" />
        <Button on:click={toggleQr} size="small" kind="tertiary"
          >{`${showQr ? "Hide QR" : "Show QR"}`}</Button
        >
      </div>
    </section>
    {#if showQr && admin_pubkey}
      <div class="qr-wrap">
        <QrCode padding={1.5} value={qrString} />
      </div>
    {/if}
    <div class="divider" />
  {/if}

  <section class="header-wrap">
    <h1 class="default-header">Default Tribes</h1>
    <!-- <small>(A maximum of 5 default tribes)</small> -->
    <small>(New users automatically added)</small>
  </section>
  <div class="divider" />

  <div class="tribes-data">
    {#if defaultTribes && defaultTribes.length}
      {#each defaultTribes as tribe}
        <section class="tribes">
          <p class="name">{tribe.name}</p>
          <button on:click={() => deleteTribe(tribe.id)} class="delete-btn">
            <svg
              viewBox="0 0 24 24"
              width="20"
              height="20"
              stroke="white"
              stroke-width="3"
              stroke-linecap="round"
            >
              <line x1="2" y1="2" x2="22" y2="22" />
              <line x1="22" y1="2" x2="2" y2="22" />
            </svg></button
          >
        </section>
      {/each}
      <div class="divider" />
    {/if}

    <section class="add-tribe-wrap">
      <label for="tribes">Add tribe</label>
      <section class="form">
        <Dropdown
          bind:selectedId={selectedTribe}
          value=""
          items={[{ id: "", text: "Select a tribe" }, ...items]}
        />
        <div class="spacer" />
        <Button
          disabled={!(selectedTribe && defaultTribes.length < 5)}
          on:click={() => addDefaultTribe(selectedTribe)}
          size="field"
          icon={Add}>Add</Button
        >
      </section>
    </section>
  </div>
</div>

<style>
  .tribes-wrap {
    padding: 1.5rem;
  }
  .header-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
  }

  .header-wrap .default-header {
    font-size: 1rem;
    font-weight: 400;
  }
  .header-wrap small {
    font-size: 0.65rem;
    color: #c6c6c6;
    margin-left: 15px;
  }
  .tribes-data .tribes {
    display: flex;
    flex-direction: row;
    align-items: center;
    padding: 0.5rem 0rem;
  }
  .tribes-data .tribes .name {
    font-size: 0.9rem;
    padding: 0;
    margin: 0;
  }
  .tribes-data .tribes .delete-btn {
    margin: 0;
    margin-left: auto;
    background-color: transparent;
    color: red;
    padding: 2px;
    border: 0;
    width: 24px;
    height: 24px;
    font-size: 0.95rem;
    font-weight: bolder;
    cursor: pointer;
    transform-origin: center center;
  }
  .tribes-data .tribes .delete-btn:hover {
    transform: scale(1.1);
  }
  .add-tribe-wrap {
    margin-top: 20px;
  }
  .add-tribe-wrap label {
    font-size: 0.8rem;
    margin-bottom: 15px;
    display: block;
    color: #c6c6c6;
  }
  .add-tribe-wrap .form {
    text-align: center;
  }
  .admin-qr-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }
  .admin-qr-label {
    font-size: 1rem;
    font-weight: 400;
  }
  .qr-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 1rem;
  }
  .btn-spacer {
    width: 0.2rem;
  }
  .relay-admin-qr-btns {
    display: flex;
    align-items: center;
  }
</style>
