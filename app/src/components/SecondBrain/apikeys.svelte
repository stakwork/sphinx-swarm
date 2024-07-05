<script lang="ts">
  import { onMount } from "svelte";
  import Password from "../input/password.svelte";
  import { get_api_token } from "../../api/swarm";

  $: x_api_token = "";

  onMount(async () => {
    const api = await get_api_token();

    x_api_token = api.x_api_token;
  });
</script>

<div class="container">
  <div class="header">
    <h2 class="title">Api Keys</h2>
  </div>
  <div class="content">
    <div class="api-container">
      <p class="api-title">API TOKEN</p>
      <div class="password_container">
        <Password
          value={x_api_token}
          onInput={() => {}}
          label=""
          readonly={true}
        />
      </div>
    </div>
  </div>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    padding-left: 2.25rem;
    padding-right: 2.25rem;
  }

  .header {
    display: flex;
    padding: 1.8125rem 0rem;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }

  .title {
    color: #fff;
    font-family: "Barlow";
    font-size: 1.125rem;
    font-style: normal;
    font-weight: 700;
    line-height: 1rem; /* 88.889% */
    letter-spacing: 0.01125rem;
  }

  .content {
    display: flex;
    flex-direction: column;
  }

  .api-container {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .api-title {
    font-weight: 500;
    font-family: "Barlow";
  }

  .password_container {
    width: 70%;
  }
</style>
