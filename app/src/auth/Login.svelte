<script lang="ts">
  import {
    Button,
    TextInput,
    Loading,
    Form,
    ToastNotification,
  } from "carbon-components-svelte";
  import Icon from "carbon-icons-svelte/lib/Login.svelte";
  import * as api from "../api";
  import { onMount, onDestroy } from "svelte";
  import { contructQrString } from "../helpers";
  // import { saveUserToStore } from "../store";

  export let saveUserToStore = (_a: string) => {};

  $: username = "";
  $: password = "";
  $: qrString = "";
  $: challenge = "";

  $: addDisabled = !username || !password;

  let loading = false;
  let sphinxSignError = false;
  let interval;

  async function login() {
    try {
      loading = true;
      const result = await api.swarm.login(username, password);

      if (result) {
        saveUserToStore(result.token);
        username = "";
        password = "";
      }
      loading = false;
    } catch (_) {
      loading = false;
    }
  }

  async function startPolling() {
    let i = 0;
    interval = setInterval(async () => {
      try {
        const response = await api.swarm.get_challenge_status(challenge);
        if (response.success) {
          challenge = "";
          saveUserToStore(response.token);
          loading = false;
          if (interval) clearInterval(interval);
        }

        if (!response.success && response.message === "unauthorized") {
          challenge = "";
          loading = false;
          sphinxSignError = true;
          if (interval) clearInterval(interval);
          setTimeout(() => {
            sphinxSignError = false;
          }, 20000);
        }

        i++;
        if (i > 100) {
          loading = false;
          if (interval) clearInterval(interval);
        }
      } catch (e) {
        loading = false;
        console.log("Auth interval error", e);
      }
    }, 3000);
  }

  async function loginWithSphinx(e) {
    try {
      loading = true;
      startPolling();
    } catch (error) {
      loading = false;
    }
  }

  onMount(async () => {
    const result = await api.swarm.get_challenge();
    if (result) {
      challenge = result.challenge;
      qrString = contructQrString(result.challenge);
    }
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });
</script>

<main>
  <div class="logo-wrap">
    <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
    <span class="stack-title">Sphinx</span>
  </div>
  <div class="container">
    {#if loading}
      <Loading />
    {:else}
      <section class="login-wrap">
        {#if sphinxSignError}
          <div class="toast_container">
            <ToastNotification
              fullWidth
              title="Error"
              subtitle="You are not the authorized admin"
            />
          </div>
        {/if}
        <h3 class="header-text">Login to Sphinx Swarm</h3>
        <Form on:submit>
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
              type="submit"
              class="peer-btn"
              on:click={login}
              size="field"
              icon={Icon}>Login</Button
            ></center
          >
        </Form>
        <div class="sphinx-login-container">
          <a href={qrString} class="sphinx-button" on:click={loginWithSphinx}
            >Login with Sphinx</a
          >
        </div>
      </section>
    {/if}
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
    min-height: 85vh;
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

  .toast_container {
    margin-bottom: 1rem;
  }

  .sphinx-login-container {
    display: flex;
    width: 100%;
    justify-content: center;
    align-items: center;
    padding: 2rem;
  }

  .sphinx-button {
    padding: 1rem 2rem;
    font-size: 1rem;
    border: none;
    outline: none;
    font-weight: 500;
    border-radius: 0.2rem;
    cursor: pointer;
    background-color: #618aff;
    color: white;
    text-decoration: none;
  }

  .sphinx-button:hover {
    background-color: #4d6ecc;
  }
</style>
