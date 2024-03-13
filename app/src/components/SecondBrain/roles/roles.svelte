<script lang="ts">
  import SetSuperAdmin from "./setSuperAdmin.svelte";
  import { boltwallSuperAdminPubkey } from "../../../store";
</script>

<div class="container">
  {#if !$boltwallSuperAdminPubkey}
    <SetSuperAdmin />
  {/if}
</div>

<style>
</style>
