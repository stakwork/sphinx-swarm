<script lang="ts">
  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import Copy from "carbon-icons-svelte/lib/Copy.svelte";
  import * as api from "../api";
  import * as CLN from "../api/cln";
  import {
    lightningAddresses,
    onChainAddressGeneratedForOnboarding,
    finishedOnboarding,
    copiedAddressForOnboarding,
    lndBalances,
    unconfirmedBalance,
  } from "../store";
  import { onDestroy, onMount } from "svelte";
  import {
    parseClnListFunds,
    parseUnconfirmedClnBalance,
  } from "../helpers/cln";

  export let tag = "";
  export let type = "";

  let balInterval;

  async function newAddress() {
    let new_addy;
    if (type === "Cln") {
      new_addy = await api.cln.new_address(tag);
    } else {
      new_addy = await api.lnd.new_address(tag);
      if (new_addy && !$finishedOnboarding.hasChannels) {
        onChainAddressGeneratedForOnboarding.update(() => true);
      }
    }
    if (!new_addy) return;
    lightningAddresses.update((addys) => {
      return { ...addys, [tag]: new_addy };
    });
  }

  onMount(() => {
    getBalance();

    //Polling Get Balance
    balInterval = setInterval(getBalance, 20000);
  });

  onDestroy(() => {
    if (balInterval) clearInterval(balInterval);
  });

  async function getBalance() {
    if (type === "Lnd") {
      const balance = await api.lnd.get_balance(tag);
      updateConfirmedBalance(balance?.confirmed_balance);
      updateUnconfirmedBalance(balance?.unconfirmed_balance);
    } else if (type === "Cln") {
      const funds = await CLN.list_funds(tag);
      const thechans = await CLN.list_peer_channels(tag);
      const balance = parseClnListFunds(funds, thechans);
      const unconfirmed_balance = parseUnconfirmedClnBalance(funds);
      updateConfirmedBalance(balance);
      updateUnconfirmedBalance(unconfirmed_balance);
    }
  }

  function updateConfirmedBalance(balance) {
    if (lndBalances.hasOwnProperty(tag) && lndBalances[tag] === balance) return;
    lndBalances.update((n) => {
      return { ...n, [tag]: balance };
    });
  }

  function updateUnconfirmedBalance(balance) {
    if (
      unconfirmedBalance.hasOwnProperty(tag) &&
      unconfirmedBalance[tag] === balance
    )
      return;
    unconfirmedBalance.update((n) => {
      return { ...n, [tag]: balance };
    });
  }
  $: myNewAddy = $lightningAddresses[tag];

  function copyAddressToClipboard() {
    navigator.clipboard.writeText(myNewAddy);
    copiedAddressForOnboarding.update(() => true);
  }
</script>

<div class="wrap">
  <div class="confirmed_balance_container">
    <p class="confirmed_balance">Confirmed Balance:</p>
    <p class="confirmed_amount">{$lndBalances[tag] || 0}</p>
  </div>
  <aside class="address-wrap">
    <div class="address">
      <section class="input-wrap">
        <label for="address">Address (Generate or copy address)</label>
        <aside class="data-wrap">
          <input
            name="address"
            bind:value={myNewAddy}
            placeholder="Address"
            readonly
          />
          <button class="copy-btn" on:click={copyAddressToClipboard}
            ><Copy class="copy-icon" size={24} /></button
          >
        </aside>
      </section>
    </div>

    <aside class="spacer" />
    <Button on:click={newAddress} size="field" icon={Add}
      >Generate Address</Button
    >
  </aside>
  <div class="unconfirmed_balance_container">
    <p class="unconfirmed_balance">Unconfirmed Balance:</p>
    <p class="unconfirmed_amount">{$unconfirmedBalance[tag] || 0}</p>
  </div>
</div>

<style>
  .wrap {
    padding: 1.5rem;
  }

  .confirmed_balance {
    font-size: 0.875rem;
    color: #527931;
  }

  .confirmed_amount {
    font-size: 1rem;
    font-weight: 500;
    color: #59b708;
  }

  .unconfirmed_balance {
    font-size: 0.875rem;
    color: #8d562e;
    text-align: right;
  }

  .unconfirmed_amount {
    font-size: 1.125rem;
    font-weight: 500;
    color: #f47d27;
    text-align: right;
  }

  .confirmed_balance_container {
    margin-bottom: 2rem;
  }
  .unconfirmed_balance_container {
    margin-top: 2rem;
    margin-left: auto;
  }
  .address-wrap {
    margin-top: 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .address {
    width: 100%;
  }
  .address .input-wrap {
    width: 100%;
  }
  .address .input-wrap input {
    height: 40px;
    padding: 5px 15px;
    background: transparent;
    color: #fff;
    font-size: 0.9rem;
    width: 97%;
    border: 1.5px solid #fff;
    border-top-left-radius: 2px;
    border-bottom-left-radius: 2px;
  }
  .address .input-wrap label {
    font-size: 0.85rem;
    margin-bottom: 10px;
    display: block;
  }

  .address .input-wrap .data-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
  }

  .copy-btn {
    background: transparent;
    padding: 0;
    margin: 0;
    border: 0;
    color: #fff;
    width: 50px;
  }
</style>
