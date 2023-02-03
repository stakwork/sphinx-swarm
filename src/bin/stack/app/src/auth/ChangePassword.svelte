<script>
  import { Button, TextInput, Loading, Form } from "carbon-components-svelte";
  import Icon from "carbon-icons-svelte/lib/Password.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import * as api from "../api";
  import { activeUser } from "../store";

  export let back = () => {};

  $: old_pass = "";
  $: password = "";
  $: confirm_pass = "";

  $: addDisabled =
    !old_pass ||
    !password ||
    !confirm_pass ||
    password !== confirm_pass;

  let loading = false;

  async function change() {
    try {
      loading = true;

      const result = await api.swarm.update_password(password, old_pass, $activeUser);

      if (result) {
        old_pass = "";
        password = "";
        confirm_pass = "";
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
            labelText={"Old Password"}
            placeholder={"Enter your old password"}
            type="password"
            bind:value={old_pass}
          />
          <div class="spacer" />
          <TextInput
            labelText={"New Password"}
            placeholder={"Enter your new password"}
            type="password"
            bind:value={password}
          />
          <div class="spacer" />
          <TextInput
            labelText={"Confirm Password"}
            placeholder={"Confirm your password"}
            type="password"
            bind:value={confirm_pass}
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
