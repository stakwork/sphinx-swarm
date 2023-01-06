<script>
  import Users from "./Users.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import { selectedNode } from "../store";
  import Controls from "../controls/Controls.svelte";
  import { controls } from "../controls/controls";

  export let tag = "";
</script>

<Tabs>
  <Tab label="Users" />
  <Tab label="Configuration" />
  <svelte:fragment slot="content">
    <TabContent><Users /></TabContent>
    <TabContent>
      <Controls ctrls={$selectedNode && controls["Relay"]} {tag} />
    </TabContent>
  </svelte:fragment>
</Tabs>
