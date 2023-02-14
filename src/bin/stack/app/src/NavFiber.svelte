<script>
  import { Button } from "carbon-components-svelte";

  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8001";
</script>

<div class="nav-wrapper">
  <Button target="_blank" href={link}>Open Second Brain</Button>
</div>

<style>
  .nav-wrapper {
    padding: 0px 25px;
  }
</style>
