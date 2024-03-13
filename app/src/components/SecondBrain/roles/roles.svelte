<script lang="ts">
  import SetSuperAdmin from "./setSuperAdmin.svelte";
  import { boltwallSuperAdminPubkey } from "../../../store";
  import { onMount } from "svelte";
  import { get_super_admin } from "../../../api/swarm";

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
    checkSuperAdminExist();
  });
</script>

<div class="container">
  {#if !$boltwallSuperAdminPubkey}
    <SetSuperAdmin />
  {/if}
</div>

<style>
</style>
