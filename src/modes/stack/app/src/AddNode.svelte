<script lang="ts">
  import { Button, Modal } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
//   import { selectedNode } from "./store";
  import Controls from "./Controls.svelte";
  import {controls} from "./controls";

  let open = false;
  $: ctrls = controls['NodeTypes'];
</script>

<section class="add-node-btn">
  <Button type="submit" size="field" icon={Add} on:click={() => (open = true)}
    >Add New Node</Button
  >

  <Modal
    bind:open
    modalHeading="Add Node"
    hasForm={true}
    class="new-node-modal"
    size="sm"
    primaryButtonText="Add"
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open = !open)}
  >
    <section>
        <Controls {ctrls}/>
    </section>
  </Modal>
</section>

<style>
  .add-node-btn {
    margin-left: auto;
    margin-right: 2.5rem;
  }
</style>
