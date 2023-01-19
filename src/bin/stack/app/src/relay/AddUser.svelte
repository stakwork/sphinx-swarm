<script>
    import { Button, TextInput } from "carbon-components-svelte";
    import Add from "carbon-icons-svelte/lib/Add.svelte";
    import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  
    $: pubkey = "";
  
    async function addUser() {};
  
    export let back = () => {};
  </script>
  
  <section class="add-user-wrap">
    <div class="back" on:click={back} on:keypress={() => {}}>
      <ArrowLeft size={24} />
    </div>
    <section class="user-content">
      <div class="spacer" />
      <TextInput
        labelText={"Pubkey"}
        placeholder={"Enter user pubkey"}
        bind:value={pubkey}
      />
      <div class="spacer" />
      <center
        ><Button class="peer-btn" on:click={addUser} size="field" icon={Add}
          >Add User</Button
        ></center
      >
    </section>
  </section>
  
  <style>
    .add-user-wrap {
      padding: 20px 30px;
    }
  
    .back {
      cursor: pointer;
      height: 2rem;
      width: 2rem;
    }
  </style>
  