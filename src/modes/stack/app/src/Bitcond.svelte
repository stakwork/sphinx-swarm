<script>
    import Users from "./Users.svelte";
    import { Tabs, Tab, TabContent } from "carbon-components-svelte";
    import { selectedNode } from "./store";
    import Controls from "./Controls.svelte";
    import { controls } from "./controls";
  </script>
  
  <Tabs>
    <Tab label="Info" />
    <Tab label="Configuration" />
    <svelte:fragment slot="content">
      <TabContent><Users /></TabContent>
      <TabContent>
        <Controls ctrls={$selectedNode && controls["Relay"]} />
      </TabContent>
    </svelte:fragment>
  </Tabs>