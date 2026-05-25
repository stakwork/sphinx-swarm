<script lang="ts">
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import EnvContainer from "../components/envContainer/index.svelte";
</script>

<div class="default-node-wrapper">
  <Tabs>
    <Tab label="Advance" />
    <svelte:fragment slot="content">
      <TabContent><EnvContainer /></TabContent>
    </svelte:fragment>
  </Tabs>
</div>

<style>
  .default-node-wrapper {
    width: 100%;
  }
</style>
