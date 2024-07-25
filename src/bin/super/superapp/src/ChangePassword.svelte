<script>
  import {
    Button,
    TextInput,
    Loading,
    Form,
    InlineNotification,
  } from "carbon-components-svelte";
  import Icon from "carbon-icons-svelte/lib/Password.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import * as api from "../../../../../app/src/api";
  import { activeUser, logoutUser } from "./store";

  export let back = () => {};

  $: old_pass = "";
  $: password = "";
  $: confirm_pass = "";
  let show_notification = false;

  $: addDisabled =
    !old_pass || !password || !confirm_pass || password !== confirm_pass;

  let loading = false;

  async function change() {
    try {
      loading = true;

      const result = await api.swarm.update_password(
        password,
        old_pass,
        $activeUser
      );

      if (result) {
        show_notification = true;

        old_pass = "";
        password = "";
        confirm_pass = "";
      }
      loading = false;
      setTimeout(() => {
        back();
        logoutUser();
      }, 3000);
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

        {#if show_notification}
          <InlineNotification
            lowContrast
            kind="success"
            title="Success:"
            subtitle="Your password has been changed."
            timeout={3000}
            on:close={(e) => {
              e.preventDefault();
              show_notification = false;
            }}
          />
        {/if}

        <Form on:submit>
          <div class="input_container">
            <TextInput
              labelText={"Old Password"}
              placeholder={"Enter your old password"}
              type="password"
              bind:value={old_pass}
            />
          </div>
          <div class="input_container">
            <TextInput
              labelText={"New Password"}
              placeholder={"Enter your new password"}
              type="password"
              bind:value={password}
            />
          </div>

          <div class="input_container">
            <TextInput
              labelText={"Confirm Password"}
              placeholder={"Confirm your password"}
              type="password"
              bind:value={confirm_pass}
            />
          </div>
          <div class="btn-container">
            <Button
              disabled={addDisabled}
              on:click={change}
              class="peer-btn"
              size="field"
              type="submit"
              icon={Icon}>Change Password</Button
            >
          </div>
        </Form>
      </section>
    {/if}
  </div>
</main>

<!-- on:click={change} -->

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

  .input_container {
    margin-bottom: 1.5rem;
  }

  .btn-container {
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
