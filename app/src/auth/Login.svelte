<script lang="ts">
  import { ToastNotification } from "carbon-components-svelte";
  import * as api from "../api";
  import { onMount, onDestroy } from "svelte";
  import { contructQrString } from "../helpers";
  // import { saveUserToStore } from "../store";
  import Input from "../components/input/input.svelte";
  import Password from "../components/input/password.svelte";

  export let saveUserToStore = (_a: string) => {};

  $: username = "";
  $: password = "";
  $: qrString = "";
  $: challenge = "";
  $: message = "";

  $: addDisabled = !username || !password;

  let loading = false;
  let sphinx_app_loading = false;
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
          sphinx_app_loading = false;
          if (interval) clearInterval(interval);
        }

        if (!response.success && response.message === "unauthorized") {
          challenge = "";
          sphinx_app_loading = false;
          sphinxSignError = true;
          message = "You are not the authorized admin";
          if (interval) clearInterval(interval);
          setTimeout(() => {
            sphinxSignError = false;
          }, 20000);
        }

        i++;
        if (i > 100) {
          sphinx_app_loading = false;
          sphinxSignError = true;
          message = "Timeout, please try again";
          if (interval) clearInterval(interval);
          setTimeout(() => {
            sphinxSignError = false;
          }, 20000);
        }
      } catch (e) {
        sphinx_app_loading = false;
        console.log("Auth interval error", e);
      }
    }, 3000);
  }

  function handleUsernameInput(value) {
    username = value;
  }

  function handlePasswordInput(value) {
    password = value;
  }

  async function loginWithSphinx(e) {
    try {
      sphinx_app_loading = true;
      startPolling();
    } catch (error) {
      sphinx_app_loading = false;
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

<main class="container">
  <div class="image_container">
    <div class="welcome_container">
      <img src="swarm/old_logo.svg" alt="logo" />
      <h2 class="welcome_text">
        Welcome to <span class="app_name">Sphinx Swarm</span>
      </h2>
    </div>
  </div>
  <div class="sign_contianer">
    <div class="login_inner_container">
      {#if sphinxSignError}
        <div class="toast_container">
          <ToastNotification fullWidth title="Error" subtitle={message} />
        </div>
      {/if}
      <h2 class="login_text">Login</h2>
      <div class="form_container">
        <div class="inputs_container">
          <Input
            label="Username"
            placeholder="Enter Username ..."
            bind:value={username}
            onInput={handleUsernameInput}
          />

          <Password
            label="Password"
            placeholder="Enter Password ..."
            bind:value={password}
            onInput={handlePasswordInput}
          />
        </div>
        <div class="submit_btn_container">
          <button
            disabled={loading || addDisabled || sphinx_app_loading}
            on:click={login}
            class="submit_btn"
          >
            {#if loading === true}
              <div class="loading-spinner"></div>
            {:else}
              Login
            {/if}</button
          >
        </div>
        <div class="alt_info">
          <div class="line"></div>
          <p class="or">OR</p>
          <div class="line"></div>
        </div>
        <div class="sphinx_btn_container">
          <button
            disabled={!challenge || !qrString || sphinx_app_loading || loading}
            class="sphinx_btn"
            on:click={loginWithSphinx}
          >
            {#if sphinx_app_loading}
              <div class="sphinx_loading-spinner_container">
                <div class="sphinx-loading-spinner"></div>
              </div>
            {:else}
              <a href={qrString} class="sphinx_link">
                <img
                  src="swarm/sphinx_logo.svg"
                  alt="sphinx"
                  class="sphinx_logo"
                />Login With Sphinx
              </a>
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>
</main>

<style>
  main {
    height: 100vh;
    width: 100vw;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
  }

  .toast_container {
    margin-bottom: 2rem;
  }

  .image_container {
    background-image: url("../swarm/login_cover.svg");
    background-size: cover;
    background-position: center;
    background-color: #16171d;
    width: 100%;
  }

  .welcome_container {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 7.625rem;
    gap: 2.75rem;
  }

  .welcome_text {
    font-family: "Barlow";
    font-weight: 300;
    font-size: 2.25rem;
    line-height: 2.7rem;
    color: #ffffff;
  }

  .app_name {
    font-family: "Barlow";
    font-weight: 700;
  }

  .sign_contianer {
    background-color: #23252f;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .login_text {
    margin-bottom: 2rem;
    font-family: "Barlow";
    font-size: 1.875rem;
    font-weight: 700;
    line-height: 1rem;
  }

  .login_inner_container {
    width: 20.625rem;
  }

  .form_container {
    display: flex;
    flex-direction: column;
  }

  .inputs_container {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .submit_btn_container {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2.12rem;
    margin-top: 2rem;
  }

  .submit_btn {
    color: #16171d;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 135.714% */
    padding: 0.75rem 1rem;
    border-radius: 0.375rem;
    background: #fff;
    border: none;
    width: 100%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .submit_btn:disabled {
    cursor: not-allowed;
  }

  .loading-spinner {
    border: 2px solid #16171d;
    border-top: 2px solid #fff;
    border-radius: 50%;
    width: 1.125rem;
    height: 1.125rem;
    animation: spin 1s linear infinite;
  }

  .alt_info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 1.75rem;
    margin-bottom: 1.75rem;
  }

  .line {
    width: 8.12rem;
    height: 1px;
    background-color: #6b7a8d;
  }

  .or {
    color: #6b7a8d;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 500;
    line-height: 1.1875rem; /* 135.714% */
  }

  .sphinx_btn_container {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    gap: 1.25rem;
    margin-bottom: 4rem;
  }

  .sphinx_btn {
    display: flex;
    align-items: center;
    border-radius: 0.375rem;
    background: #618aff;
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 1rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 135.714% */
    width: 100%;
    border: none;
    cursor: pointer;
  }

  .sphinx-loading-spinner {
    border: 2px solid #fff;
    border-top: 2px solid #618aff;
    border-radius: 50%;
    width: 1.125rem;
    height: 1.125rem;
    animation: spin 1s linear infinite;
  }

  .sphinx_loading-spinner_container {
    display: flex;
    padding: 0.8125rem;
    width: 100%;
    align-items: center;
    justify-content: center;
  }

  .sphinx_link {
    display: flex;
    align-items: center;
    text-decoration: none;
    color: #fff;
    padding: 0.8125rem;
  }

  .sphinx_btn:disabled {
    cursor: not-allowed;
  }

  .sphinx_logo {
    width: 1.371rem;
    height: 1.3125rem;
    margin-right: 3.945rem;
  }
</style>
