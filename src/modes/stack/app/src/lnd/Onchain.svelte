<script lang="ts">
  export let tag = "";
  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import Copy from "carbon-icons-svelte/lib/Copy.svelte";
  import * as api from "../api";
  import { lightningAddresses } from "../store";

  async function newAddress() {
    let new_addy = await api.lnd.new_address(tag);
    lightningAddresses.update((addys) => {
      return { ...addys, [tag]: new_addy };
    });
  }

  $: myNewAddy = $lightningAddresses[tag];

  function copyAddressToClipboard() {
    navigator.clipboard.writeText(myNewAddy);
  }
</script>

<div class="wrap">
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
</div>

<style>
  .wrap {
    padding: 1.5rem;
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
