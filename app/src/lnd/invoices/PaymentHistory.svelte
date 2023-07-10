<script lang="ts">
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import Transaction from "../../components/transaction.svelte";
  export let tag = "";
  export let type = "";
</script>

<main>
  <Tabs>
    <Tab label="Sent Payments" />
    <Tab label="Recieved Payments" />
    <svelte:fragment slot="content">
      <TabContent>
        <Transaction {tag} {type} paymentType={"sent"} />
      </TabContent>
      <TabContent>
        <Transaction {tag} {type} paymentType={"recieved"} />
      </TabContent>
    </svelte:fragment>
  </Tabs>
</main>

<style>
  main {
    padding: 1.5rem 1.8rem;
    margin-top: 0.1rem;
  }
</style>
