<script>
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import Channels from "./Channels.svelte";
  import Invoices from "./Invoices.svelte";
  import Onchain from "./Onchain.svelte";
  import { finishedOnboarding } from "../store";

  export let tag = "";
  export let type = "";
  $: selected = 0;
  $: $finishedOnboarding, selectCurrentTab();
  function selectCurrentTab() {
    if (!$finishedOnboarding.hasChannels) {
      selected = 2;
    }
  }
</script>

<div class="lnd-tabs-wrap">
  <Tabs bind:selected>
    <Tab label="Channels" />
    <Tab label="Invoices" />
    <Tab label="Onchain" />
    <svelte:fragment slot="content">
      <TabContent><Channels {tag} {type} /></TabContent>
      <TabContent>
        <Invoices {tag} {type} />
      </TabContent>
      <TabContent>
        <Onchain {tag} {type} />
      </TabContent>
    </svelte:fragment>
  </Tabs>
</div>
