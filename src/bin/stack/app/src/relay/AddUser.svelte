<script>
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import * as api from "../api";
  import { relayBalances } from "../store";
  import { formatSatsNumbers } from "../helpers";
  import { onMount } from "svelte";

  export let back = () => {};
  export let tag = "";

  $: initialSats = 0;

  $: balance = $relayBalances.hasOwnProperty(tag)
    ? $relayBalances[tag]["full_balance"]
    : 0;

  $: addDisabled = initialSats > balance;

  let errorMsg = "";
  let calling = false;
  async function addUser() {
    calling = true;
    const u = await api.relay.add_user(tag, initialSats || null);
    if (u) back();
    else {
      initialSats = 0;
      errorMsg = "Failed to add user";
      setTimeout(() => {
        errorMsg = "";
      }, 1234);
    }
    calling = false;
    // back();
  }

  async function getBalance() {
    const balance = await api.relay.get_balance(tag);
    if (relayBalances.hasOwnProperty(tag) && relayBalances[tag] === balance)
      return;

    relayBalances.update((n) => {
      return { ...n, [tag]: balance };
    });
  }

  onMount(() => {
    getBalance();
  });
</script>

<section class="add-user-wrap">
  <div class="back" on:click={back} on:keypress={() => {}}>
    <ArrowLeft size={24} />
  </div>
  <div class="balance-wrap">
    <section class="value-wrap">
      <h3 class="title">CHANNELS BALANCE</h3>
      <h3 class="value">{formatSatsNumbers(balance)}</h3>
    </section>
  </div>
  <section class="user-content">
    <div class="spacer" />
    <TextInput
      labelText={"Satoshis to Allocate (optional)"}
      placeholder={"Enter amount in sats"}
      type="number"
      bind:value={initialSats}
    />
    <div class="spacer" />
    <center>
      <Button
        class="peer-btn"
        on:click={addUser}
        size="field"
        icon={Add}
        disabled={errorMsg || calling || addDisabled ? true : false}
      >
        Add User
      </Button>
    </center>
    {#if errorMsg}
      <center class="error">
        {errorMsg}
      </center>
    {/if}
  </section>
</section>

<style>
  .add-user-wrap {
    padding: 10px 30px;
  }

  .back {
    cursor: pointer;
    height: 2rem;
    width: 2rem;
  }
  .error {
    font-size: 0.8rem;
    margin-top: 1.5rem;
  }
  .balance-wrap {
    margin: 0.35rem 0;
  }
</style>
