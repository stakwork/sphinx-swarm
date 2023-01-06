<script lang="ts">
  import { Button } from "carbon-components-svelte";
  import Mine from "carbon-icons-svelte/lib/VirtualMachine.svelte";
  import * as api from "./api";

  export let tag = "";
  export let getBitcoinInfo = () => {};

  let open = false;
  $: blockLen = 6;
  $: ok = blockLen;

  async function mine() {
    const result = await api.btc.test_mine(tag, blockLen);
    if (result) {
      open = false;

      // Set values to default
      blockLen = 6;

      // Get new Bitcoin info
      getBitcoinInfo();
    }
  }
</script>

<section class="mine-blocks-btn">
  <aside class="mine-wrap">
    <input
      bind:value={blockLen}
      type="number"
      placeholder="Enter number of blocks"
    />
    <Button on:click={mine} size="field" icon={Mine}>Mine blocks</Button>
  </aside>
</section>

<style>
  .mine-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
  }
  .mine-wrap input {
    height: 45px;
    margin-right: 20px;
    padding: 5px 10px;
    background: transparent;
    color: #FFF;
    font-size: 1rem;
    width: 200px;
    border: 1.5px solid #FFF;
    border-radius: 2px;
  }
</style>
