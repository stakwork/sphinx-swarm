<script lang="ts">
  import { onMount } from "svelte";
	import { createScene } from "./scene";

  let el;

  onMount(() => {
    createScene(el)
  });

</script>

<main >
  <header>
    <div class="lefty logo-wrap">Sphinx Stack</div>
  </header>
	<canvas bind:this={el}></canvas>
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
  /* tell our scene container to take up the full page */
  position: absolute;
  width: 100%;
  height: 100%;

  /*
    Set the container's background color to the same as the scene's
    background to prevent flashing on load
  */
  background-color: skyblue;
  }
  .lefty {
    width: 15rem;
    max-width: 15rem;
    height: 100%;
    border-right: 1px dashed #bfbfbf;
  }
</style>
