<script>
  import { Button, TextInput } from "carbon-components-svelte";
  import Icon from "carbon-icons-svelte/lib/Login.svelte";
  import * as api from "./api";

  $: username = "";
  $: password = "";

  $: addDisabled = !username || !password;

  async function login() {
    if (await api.swarm.login(username, password)) {
      username = "";
      password = "";
    }
  }
</script>

<main>
  <div class="logo-wrap">
    <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
    <span class="stack-title">Sphinx</span>
  </div>
  <div class="container">
    <section class="login-wrap">
      <h3 class="header-text">Login to Sphinx Swarm</h3>
      <TextInput
        labelText={"Username"}
        placeholder={"Enter username"}
        bind:value={username}
      />
      <div class="spacer" />
      <TextInput
        labelText={"Password"}
        placeholder={"Enter password"}
        type={"password"}
        bind:value={password}
      />
      <div class="spacer" />
      <center
        ><Button
          disabled={addDisabled}
          class="peer-btn"
          on:click={login}
          size="field"
          icon={Icon}>Login</Button
        ></center
      >
    </section>
  </div>
</main>

<style>
  main {
    height: 100vh;
    width: 100vw;
    background: #1a242e;
  }
  .container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 88vh;
  }
  .logo-wrap {
    padding: 22px;
    margin-left: 35px;
    display: flex;
    align-items: center;
  }
  .logo-wrap .logo {
    border-radius: 50%;
    width: 55px;
    height: 55px;
  }
  .logo-wrap .stack-title {
    margin-left: 12px;
  }
  .login-wrap {
    width: 35vw;
    text-align: left;
  }
  .header-text {
    font-size: 1.25rem;
    font-size: 900;
    margin-bottom: 35px;
  }
</style>
