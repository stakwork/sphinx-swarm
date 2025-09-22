<script lang="ts">
  import Remotes from "./Remotes.svelte";
  import Login from "../../../../../app/src/auth/Login.svelte";
  import { activeUser, saveUserToStore, logoutUser } from "./store";
  import User from "carbon-icons-svelte/lib/User.svelte";
  import {
    OverflowMenu,
    OverflowMenuItem,
    Tabs,
    Tab,
    TabContent,
  } from "carbon-components-svelte";
  import ChangePassword from "./ChangePassword.svelte";
  import ViewNodes from "./ViewNodes.svelte";
  import LightningBot from "./LightningBot.svelte";
  import Logs from "./logs.svelte";
  import AnthropicKeys from "./AnthropicKeys.svelte";
  let page = "main";

  async function backToMain() {
    page = "main";
  }

  async function viewNode() {
    page = "view_nodes";
  }

  function handleChangePassword() {
    page = "change_password";
  }
</script>

<main>
  {#if !$activeUser}
    <Login {saveUserToStore} />
  {:else}
    <header>
      <div class="lefty logo-wrap">
        <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
        <span class="stack-title">Sphinx Superadmin</span>
      </div>
      <section class="menu-btn">
        <OverflowMenu icon={User} flipped>
          <OverflowMenuItem on:click={logoutUser} text="Logout" />
          <OverflowMenuItem
            on:click={handleChangePassword}
            text="Change Password"
          />
        </OverflowMenu>
      </section>
    </header>
    <div class="body">
      {#if page === "change_password"}
        <ChangePassword back={backToMain} />
      {:else}
        <Tabs width="100%">
          <Tab label="Swarms" />
          <Tab label="Lightning" />
          <Tab label="Logs" />
          <Tab label="Anthropic Keys" />
          <svelte:fragment slot="content">
            <TabContent>
              <div>
                {#if page === "view_nodes"}
                  <ViewNodes back={backToMain} />
                {:else}
                  <Remotes {viewNode} />
                {/if}
              </div>
            </TabContent>
            <TabContent><LightningBot /></TabContent>
            <TabContent><Logs /></TabContent>
            <TabContent><AnthropicKeys /></TabContent>
          </svelte:fragment>
        </Tabs>
      {/if}
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
    width: 100%;
    background: #1a242e;
    align-items: center;
    justify-content: space-between;
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
    height: calc(100vh - 4.2rem);
    width: 100%;
    flex-direction: column;
  }
  .stack-title {
    color: white;
    margin-left: 0.5rem;
    font-size: 1.2rem;
  }
  .menu-btn {
    margin-right: 2rem;
  }
</style>
