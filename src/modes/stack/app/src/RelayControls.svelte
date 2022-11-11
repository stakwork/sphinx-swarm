<script>
  import Users from "./Users.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import { selectedNode } from "./store";
  import Ctrl from "./Ctrl.svelte";
  import { controls } from "./controls";

  $: ctrls = $selectedNode && controls["Relay"];
</script>

<Tabs>
  <Tab label="Users" />
  <Tab label="Configuration" />
  <svelte:fragment slot="content">
    <TabContent><Users /></TabContent>
    <TabContent>
      <div class="controls">
        {#each ctrls as ctrl}
          <Ctrl {...ctrl} />
          <div class="spacer" />
        {/each}
      </div>
    </TabContent>
  </svelte:fragment>
</Tabs>

<style>
  .spacer {
    margin-bottom: 1rem;
  }
</style>
