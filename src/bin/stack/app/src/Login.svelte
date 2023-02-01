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
  <section class="login-wrap">
    <TextInput
      labelText={"Username"}
      placeholder={"Enter channel amount"}
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
        icon={Icon}>Sign in</Button
      ></center
    >
  </section>
</main>

<style>
</style>
