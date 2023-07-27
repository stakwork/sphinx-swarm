<script lang="ts">
  import Login from "carbon-icons-svelte/lib/Login.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import CopyIcon from "carbon-icons-svelte/lib/Copy.svelte";
  import Save from "carbon-icons-svelte/lib/Save.svelte";
  import QrCode from "svelte-qrcode";
  import DotWrap from "../components/DotWrap.svelte";
  import Dot from "../components/Dot.svelte";
  import type { User } from "./users";
  import { node_host } from "../store";
  import { Button } from "carbon-components-svelte";

  export let select = (pubkey: string) => {};
  export let user: User;
  export let selected = false;

  const signedUp = user.alias ? true : false;

  function mainSelect() {
    if (!selected) select(user.public_key);
  }
  function back() {
    select(null);
  }
  function copyToClipboard(value) {
    navigator.clipboard.writeText(value);
  }

  $: qrString = `connect::${$node_host}::${user.public_key}`;

  function saveQr() {
    console.log("save qr");
    let wrap = document.getElementsByClassName("qr-wrap")[0];
    let img = wrap.firstChild;
    let b64 = img && (img as any).getAttribute("src");
    if (b64) downloadURI(b64, "sphinx_invite.png");
  }
  function downloadURI(uri, name) {
    var link = document.createElement("a");
    link.download = name;
    link.href = uri;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    // delete link;
  }
</script>

<div
  class={`user ${selected && "selected"}`}
  on:click={mainSelect}
  on:keypress={() => {}}
>
  <div class="top">
    <div class="top-left">
      {#if selected}
        <div class="back" on:click={back} on:keypress={() => {}}>
          <ArrowLeft size={24} />
        </div>
      {/if}
      <DotWrap>
        <Dot color={`${signedUp ? "#52B550" : "grey"}`} />
      </DotWrap>
      {#if user.alias}
        <div class="alias">{user.alias}</div>
      {:else}
        <div class="empty-alias" />
      {/if}
    </div>
    <div class="signed-up" style={`opacity:${signedUp ? 1 : "0.5"}`}>
      <Login size={16} />
      <span>{`${signedUp ? "" : "Not "}Signed Up`}</span>
    </div>
  </div>
  {#if selected}
    <div class="fields">
      <p class="user-values-title">Pubkey</p>
      <section class="value-wrap">
        <p class="user-value">{user.public_key}</p>
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <span on:click={() => copyToClipboard(user.public_key)}
          ><CopyIcon size={16} class="copy-icon" /></span
        >
      </section>
      {#if user.route_hint}
        <p class="user-values-title">Route hint</p>
        <section class="value-wrap">
          <p class="user-value">{user.route_hint}</p>
          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <span on:click={() => copyToClipboard(user.route_hint)}
            ><CopyIcon size={16} class="copy-icon" /></span
          >
        </section>
      {/if}
      <p class="user-values-title">Invite QR code</p>
      <div class="qr-wrap">
        <QrCode padding={1.5} value={qrString} size={230} />
      </div>
      <div class="qr-btns">
        <Button
          kind="tertiary"
          size="field"
          icon={CopyIcon}
          on:click={() => copyToClipboard(qrString)}>Copy</Button
        >
        <Button kind="tertiary" size="field" icon={Save} on:click={saveQr}
          >Save</Button
        >
      </div>
    </div>
  {:else}
    <div class="pubkey collapsed">
      {user.public_key}
    </div>
  {/if}
</div>

<style>
  .user {
    font-size: 1rem;
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 0.8rem 0;
  }
  .user:not(.selected) {
    cursor: pointer;
  }
  .user:hover:not(.selected) {
    background: #131b23;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .signed-up {
    display: flex;
    align-items: center;
    text-transform: uppercase;
    font-size: 0.6rem;
    margin-right: 1rem;
  }
  .signed-up span {
    margin-left: 0.2rem;
  }
  .pubkey {
    color: grey;
    margin-bottom: 0.5rem;
    font-size: 0.7rem;
    max-width: 88%;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
  }

  .balance {
    font-size: 0.8rem;
    color: #ddd;
  }
  .back {
    cursor: pointer;
    margin-left: 1rem;
    height: 2rem;
    width: 2rem;
    border-radius: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .back:hover {
    background: #070b0e;
  }
  .top-left {
    display: flex;
    align-items: center;
  }
  .collapsed {
    margin-left: 2.3rem;
  }
  .fields {
    padding: 1.5rem;
  }
  .user-values-title {
    margin: 10px 0px;
    color: grey;
    font-size: 0.78rem;
    font-weight: bold;
  }
  .value-wrap {
    display: flex;
    align-items: center;
  }
  .user-value {
    max-width: 97%;
    color: #fefefe;
    font-size: 0.7rem;
    color: #fefefe;
    overflow-x: scroll;
    white-space: nowrap;
    margin-right: 1.2%;
    padding-bottom: 11px;
  }
  .empty-alias {
    height: 0.85rem;
    width: 6rem;
    background: #283d52;
  }

  .user-delete {
    margin-left: 10px;
    background: transparent;
    padding: 0;
    border: 0;
    color: red;
  }
</style>
