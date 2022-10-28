<script lang="ts">
  import { onMount } from "svelte";
</script>

<main>
  <header>
    <div class="lefty logo-wrap">Sphinx Stack</div>
  </header>
  <div class="body">body</div>
</main>

<style>
  main {
    height: 100vh;
    width: 100vw;
    display: flex;
    background: #161616;
    flex-direction: column;
  }
  header {
    height: 4.2rem;
    display: flex;
    align-items: center;
    border-bottom: 1px dashed #bfbfbf;
  }
  .logo-wrap {
    display: flex;
    align-items: center;
  }
  .body {
    display: flex;
    height: 100%;
  }
  .lefty {
    width: 15rem;
    max-width: 15rem;
    height: 100%;
    border-right: 1px dashed #bfbfbf;
  }
</style>
