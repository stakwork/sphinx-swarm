<script>
  import App from "./App.svelte";
  import { tag, login } from "./api";

  let ctag = "";
  async function keypress(e) {
    if (e.key === "Enter") {
      if (!(await login(ctag))) {
        console.error("invalid password");
      }
    }
  }
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
