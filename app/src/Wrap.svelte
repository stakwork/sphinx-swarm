<script>
  import App from "./App.svelte";
  import { nodes, tag, login } from "./api";

  let ctag = "";
  async function keypress(e) {
    if (e.key === "Enter") {
      console.log(ctag);
      if (!(await login(ctag))) {
        console.error("invalid password");
      }
    }
  }
  $: console.log($nodes);
</script>

<main>
  {#if $tag}
    <App />
  {:else}
    <input bind:value={ctag} on:keypress={keypress} placeholder="Password" />
  {/if}
</main>

<style>
  main {
    background: black;
    padding: 1em 0.5rem;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: row;
  }
  input {
    height: 3rem;
    border-radius: 1.5rem;
    margin-top: 1rem;
    width: 16.1rem;
    padding: 0 1rem;
  }
</style>
