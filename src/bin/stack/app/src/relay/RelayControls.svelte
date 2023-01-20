<script>
  import Users from "./Users.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import Tribes from "./Tribes.svelte";
</script>

<Tabs>
  <Tab label="Users" />
  <Tab label="Configuration" />
  <svelte:fragment slot="content">
    <TabContent><Users /></TabContent>
    <TabContent>
      <Tribes />
    </TabContent>
  </svelte:fragment>
</Tabs>
