<script lang="ts">
  import { rez, send_cmd, logs, info } from "./api";
  import Btn from "./Btn.svelte";
  import { cmds } from "./cmds";
  import Cmd from "./Cmd.svelte";
  import { onMount } from "svelte";

  let textarea;
  onMount(function () {
    textarea.focus();
  });

  function send(txt: string) {
    if (txt === "clear\n") {
      return rez.set([]);
    }
    rez.update((r) => [`$ ${txt}`, ...r]);
    send_cmd(txt);
  }

  let txt = "";
  function keypress(e) {
    if (e.key === "Enter") {
      send(txt);
      txt = "";
      e.stopPropagation();
      setTimeout(() => {
        e.target.setSelectionRange(0, 0);
      }, 1);
    }
  }

  let help = true;
</script>

<section style={`width:${help ? "38" : "50"}%`}>
  <h5>Core Lightning Logs</h5>
  <p>
    {#each $logs as log}
      <div class="log">{log}</div>
    {/each}
  </p>
</section>
<section style={`width:${help ? "38" : "50"}%`}>
  <h5>Terminal</h5>
  <p>
    {#each $rez as term}
      <pre class="log">{term}</pre>
    {/each}
  </p>
  <div class="txt-wrap">
    <textarea
      bind:this={textarea}
      bind:value={txt}
      on:keypress={keypress}
      placeholder="Type commands here"
    />
    <span>$</span>
  </div>
</section>
{#if help}
  <section class="help-section">
    <h5>Node info</h5>
    <div class="break" />
    <Cmd label="Peering Port:" cmd={$info.peering} />
    <Cmd label="MQTT IP:" cmd={$info.broker_ip} />
    <Cmd label="MQTT Port:" cmd={$info.broker_port} />
    <div class="break" />
    <h5>Command Examples</h5>
    <div class="break" />
    {#each cmds as cmd}
      <Cmd {cmd} />
    {/each}
  </section>
{/if}

<Btn
  content={help ? "X" : "</>"}
  style="position:absolute;top:3px;right:1rem;"
  on:click={() => (help = !help)}
/>

<style>
  section {
    height: 100vh;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    margin: 1rem;
  }
  section:first-child {
    margin-right: 0;
  }
  h5 {
    color: grey;
    text-transform: uppercase;
    font-size: 1em;
    font-weight: 300;
    margin: 0;
    width: 100%;
    margin-top: 5px;
  }
  textarea {
    margin-left: 18px;
    border: none;
    width: calc(100% - 20px);
    height: 100%;
    overflow: scroll;
    display: inline-block;
    outline: none;
    color: white;
    font-family: Courier, sans-serif;
    font-size: 20px;
    resize: none;
    background: rgb(34, 34, 54);
  }
  p {
    width: 100%;
    height: 100%;
    max-height: 100%;
    overflow-y: auto;
    border: 1px solid white;
    background: rgb(34, 34, 54);
    color: #ddd;
    font-family: Courier, sans-serif;
    font-size: 17px;
    display: flex;
    flex-direction: column-reverse;
    align-items: center;
    margin-top: 0.5rem;
  }
  .log {
    text-align: left;
    padding: 2px 5px;
    width: 100%;
    font-size: 17px;
    margin: 1px 0;
  }
  .txt-wrap {
    border: 1px solid white;
    border-radius: 2px;
    margin-bottom: 0.86rem;
    width: 100%;
    height: 100%;
    max-height: 100px;
    position: relative;
    background: rgb(34, 34, 54);
  }
  .txt-wrap span {
    position: absolute;
    left: 8px;
    top: 10px;
    color: white;
  }
  .help-section {
    margin: 0;
    border-left: 1px solid white;
    background: rgb(34, 34, 54);
    align-items: flex-start;
    justify-content: flex-start;
    padding: 0.1rem 1rem;
    width: 24%;
  }
  .break {
    height: 1rem;
  }
</style>
