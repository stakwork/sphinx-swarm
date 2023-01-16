<script>
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import Channels from "./Channels.svelte";
  import Onchain from "./Onchain.svelte";

  export let tag = "";
</script>

<div class="lnd-tabs-wrap">
  <Tabs>
    <Tab label="Channels" />
    <Tab label="Onchain" />
    <svelte:fragment slot="content">
      <TabContent><Channels {tag} /></TabContent>
      <TabContent>
        <Onchain {tag} />
      </TabContent>
    </svelte:fragment>
  </Tabs>
</div>
