<script lang="ts">
  import Remotes from "./Remotes.svelte";
  import Login from "../../../../../app/src/auth/Login.svelte";
  import { activeUser } from "./store";
</script>

<main>
  {#if !$activeUser}
    <Login />
  {:else}
    <header>
      <div class="lefty logo-wrap">
        <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
        <span class="stack-title">Sphinx Superadmin</span>
      </div>
    </header>
    <div class="body">
      <Remotes />
    </div>
  {/if}
</main>

<style>
  main {
    height: 100vh;
    width: 100vw;
    display: flex;
    flex-direction: column;
  }
  header {
    height: 4.2rem;
    min-height: 4.2rem;
    display: flex;
    background: #1a242e;
    align-items: center;
    border-bottom: 1px solid #101317;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
  }
  .logo-wrap {
    display: flex;
    align-items: center;
  }
  .logo-wrap .logo {
    width: 70px;
    padding: 12px;
    margin-left: 2.5rem;
  }
  .body {
    display: flex;
    height: 100%;
    width: 100%;
  }
  .stack-title {
    color: white;
    margin-left: 0.5rem;
    font-size: 1.2rem;
  }
</style>
