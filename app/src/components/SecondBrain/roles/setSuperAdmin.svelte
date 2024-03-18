<script lang="ts">
  import {
    add_boltwall_admin_pubkey,
    update_admin_pubkey,
  } from "../../../api/swarm";
  import { activeUser, boltwallSuperAdminPubkey } from "../../../store";
  import Input from "../../input/input.svelte";
  let superAdminPubkey = "";
  let superAdminUsername = "";
  let isLoading = false;

  function handleAdminPubkeyInput(value) {
    superAdminPubkey = value;
  }

  function handleAdminUsernameInput(value) {
    superAdminUsername = value;
  }

  async function submitSuperAdminDetails() {
    try {
      isLoading = true;
      const result = await add_boltwall_admin_pubkey(
        superAdminPubkey,
        superAdminUsername
      );
      const swarmAdmin = await update_admin_pubkey(
        superAdminPubkey,
        $activeUser
      );

      boltwallSuperAdminPubkey.set(superAdminPubkey);
      superAdminPubkey = "";
      superAdminUsername = "";
      isLoading = false;
    } catch (error) {
      isLoading = false;
      //TODO:: Handle error properly
      console.log(
        `ERROR SETTING BOLTWALL SUPER ADMIN: ${JSON.stringify(error)}`
      );
    }
  }
</script>

<div class="container">
  <div class="inner_container">
    <div class="image_container">
      <img src="swarm/admin.svg" alt="Admin" />
    </div>
    <h2 class="heading">Set Admin</h2>
    <p class="description">Set Admin for the Second Brain</p>
    <div class="form_container">
      <div class="inputs_container">
        <Input
          label="Username"
          placeholder="Enter Username ..."
          bind:value={superAdminUsername}
          onInput={handleAdminUsernameInput}
        />

        <Input
          label="Pubkey"
          placeholder="Enter Admin Pubkey ..."
          bind:value={superAdminPubkey}
          onInput={handleAdminPubkeyInput}
        />
      </div>
      <div class="submit_btn_container">
        <button
          disabled={isLoading || !superAdminPubkey}
          on:click={submitSuperAdminDetails}
          class="submit_btn"
        >
          {#if isLoading === true}
            <div class="loading-spinner"></div>
          {:else}
            Submit
          {/if}</button
        >
      </div>
      <div class="alt_info">
        <div class="line"></div>
        <p class="or">OR</p>
        <div class="line"></div>
      </div>
      <div class="sphinx_btn_container">
        <button class="sphinx_btn">
          <img
            src="swarm/sphinx_logo.svg"
            alt="sphinx"
            class="sphinx_logo"
          />Connect With Sphinx</button
        >
        <p class="sphinx_text">To set Yourself as Superadmin</p>
      </div>
    </div>
  </div>
</div>

<style>
  .container {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
  }

  .inner_container {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 2rem;
  }

  .image_container {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .image_container img {
    height: 2.5rem;
    width: 2.5rem;
  }

  .heading {
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 1.875rem;
    font-style: normal;
    font-weight: 700;
    line-height: 1rem; /* 53.333% */
    letter-spacing: 0.01875rem;
    margin-top: 1.25rem;
  }

  .description {
    color: var(--Gray-6, #909baa);
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 114.286% */
    letter-spacing: 0.00875rem;
    margin-top: 1.25rem;
  }

  .form_container {
    display: flex;
    flex-direction: column;
    width: 16.0625rem;
    gap: 0.5rem;
    margin-top: 1.7rem;
  }

  .inputs_container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
  }

  .submit_btn_container {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2.12rem;
  }

  .loading-spinner {
    border: 2px solid #16171d; /* Light grey */
    border-top: 2px solid #fff; /* Blue */
    border-radius: 50%;
    width: 1.125rem;
    height: 1.125rem;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
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

  .submit_btn:disabled {
    cursor: not-allowed;
  }

  .alt_info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 1.75rem;
    margin-bottom: 1.75rem;
  }

  .line {
    width: 6.25rem;
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
    padding: 0.8125rem;
    border-radius: 0.375rem;
    background: #618aff;
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 135.714% */
    width: 100%;
    border: none;
    cursor: pointer;
  }

  .sphinx_logo {
    width: 1.375rem;
    height: 1.3125rem;
    margin-right: 1.84rem;
  }

  .sphinx_text {
    color: #6b7a8d;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 114.286% */
    letter-spacing: 0.00875rem;
  }
</style>
