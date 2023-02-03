<script>
  import { Button, TextInput, Loading, Form } from "carbon-components-svelte";
  import Icon from "carbon-icons-svelte/lib/Password.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import * as api from "../api";
  import { activeUser } from "../store";

  export let back = () => {};

  $: password = "";
  $: confirm_password = "";

  $: addDisabled =
    !password || !confirm_password || password !== confirm_password;

  let loading = false;

  async function change() {
    try {
      loading = true;

      const result = await api.swarm.update_password(password, $activeUser);

      console.log("Result ====", result);

      if (result) {
        password = "";
        confirm_password = "";
      }
      loading = false;
    } catch (_) {
      loading = false;
    }
  }
</script>

<main>
  <div class="back" on:click={back} on:keypress={() => {}}>
    <ArrowLeft size={32} />
  </div>
  <div class="container">
    {#if loading}
      <Loading />
    {:else}
      <section class="login-wrap">
        <h3 class="header-text">Change your password</h3>
        <Form on:submit>
          <TextInput
            labelText={"Password"}
            placeholder={"Enter password"}
            type="password"
            bind:value={password}
          />
          <div class="spacer" />
          <TextInput
            labelText={"Confirm Password"}
            placeholder={"Enter password"}
            type="password"
            bind:value={confirm_password}
          />
          <div class="spacer" />
          <center
            ><Button
              disabled={addDisabled}
              class="peer-btn"
              on:click={change}
              size="field"
              type="submit"
              icon={Icon}>Change Password</Button
            ></center
          >
        </Form>
      </section>
    {/if}
  </div>
</main>

<style>
  main {
    height: 100%;
    width: 100%;
    background: #1a242e;
  }
  .container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 85%;
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
  .back {
    margin-top: 25px;
    margin-left: 2.5rem;
    cursor: pointer;
  }
</style>
