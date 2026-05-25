<script>
  import Tribes from "./Tribes.svelte";
  import People from "./People.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import EnvContainer from "../components/envContainer/index.svelte";

  export let url = "";
</script>

<Tabs>
  <Tab label="Tribes" />
  <Tab label="People" />
  <Tab label="Advance" />
  <svelte:fragment slot="content">
    <TabContent><Tribes {url} /></TabContent>
    <TabContent><People {url} /></TabContent>
    <TabContent><EnvContainer /></TabContent>
  </svelte:fragment>
</Tabs>
