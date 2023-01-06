<script>
    import Tribes from "./Tribes.svelte";
    import People from "../People.svelte";
    import { Tabs, Tab, TabContent } from "carbon-components-svelte";

    export let url = "";
  </script>
  
  <Tabs>
    <Tab label="Tribes" />
    <Tab label="People" />
    <svelte:fragment slot="content">
      <TabContent><Tribes {url}/></TabContent>
      <TabContent><People {url}/></TabContent>
    </svelte:fragment>
  </Tabs>
  