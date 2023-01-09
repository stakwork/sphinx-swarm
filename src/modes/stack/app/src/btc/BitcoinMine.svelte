<script lang="ts">
  import { Button } from "carbon-components-svelte";
  import Mine from "carbon-icons-svelte/lib/VirtualMachine.svelte";
  import * as api from "../api";
  import { btcinfo } from "../store";

  export let tag = "";

  $: blockLen = 6;
  $: address = "";

  async function mine() {
    const result = await api.btc.test_mine(tag, blockLen, address || null);
    console.log(result);
    if (result) {
      // Set values to default
      blockLen = 6;
      address = "";
      // Get new Bitcoin info
      btcinfo.set(await api.btc.get_info(tag));
    }
  }
</script>

<section class="mine-blocks-btn">
  <aside class="mine-wrap">
    <section class="input-wrap">
      <label for="blocks">Blocks</label>
      <input
        bind:value={blockLen}
        type="number"
        placeholder="Enter number of blocks"
      />
    </section>
    <aside class="spacer" />
    <section class="input-wrap">
      <label for="blocks">Address (Optional)</label>
      <input
        bind:value={address}
        placeholder="Enter Bitcoin address (optional)"
      />
    </section>
    <aside class="spacer" />
    <Button on:click={mine} size="field" icon={Mine}>Mine blocks</Button>
  </aside>
</section>

<style>
  .mine-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .mine-wrap .input-wrap {
    width: 100%;
  }
  .mine-wrap .input-wrap input {
    height: 45px;
    margin-right: 20px;
    padding: 5px 10px;
    background: transparent;
    color: #fff;
    font-size: 1rem;
    width: 100%;
    border: 1.5px solid #fff;
    border-radius: 2px;
  }

  .mine-wrap .input-wrap label {
    font-size: 0.85rem;
    margin-bottom: 10px;
    display: block;
  }
</style>
