<script lang="ts">
  import SetSuperAdmin from "./setSuperAdmin.svelte";
  import { boltwallSuperAdminPubkey } from "../../../store";
  import { onMount } from "svelte";
  import { get_super_admin } from "../../../api/swarm";
  import UserRecord from "./userRecord.svelte";

  async function checkSuperAdminExist() {
    const result = await get_super_admin();
    const parsedResult = JSON.parse(result);
    if (
      parsedResult?.success &&
      parsedResult.message === "super admin record"
    ) {
      boltwallSuperAdminPubkey.set(parsedResult.data.pubkey);
    }
  }

  onMount(async () => {
    await checkSuperAdminExist();
  });
</script>

<div class="container">
  {#if !$boltwallSuperAdminPubkey}
    <SetSuperAdmin />
  {:else}
    <UserRecord />
  {/if}
</div>

<style>
</style>
