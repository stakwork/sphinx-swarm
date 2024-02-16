<script>
  import {
    Button,
    TextInput,
    InlineLoading,
    InlineNotification,
    DataTable,
    Dropdown,
  } from "carbon-components-svelte";
  import {
    add_boltwall_admin_pubkey,
    get_super_admin,
    add_user,
    list_admins,
    delete_sub_admin,
  } from "../api/swarm";
  import { onMount } from "svelte";
  import { shortPubkey } from "../helpers";

  $: pubkey = "";
  $: loading = false;
  $: show_notification = false;
  $: success = false;
  $: message = "";
  $: superAdminExist = false;
  $: superAdminPubkey = "";
  $: admins = [];
  $: name = "";

  let selected_role = "1";

  const items = [
    { id: "1", text: "Select Role" },
    { id: "2", text: "Sub Admin" },
    { id: "3", text: "Member" },
  ];

  async function setSuperAdmin() {
    const result = await add_boltwall_admin_pubkey(pubkey, name);
    const parsedResult = JSON.parse(result);
    success = parsedResult.success || false;
    message = parsedResult.message;
    superAdminExist = true;
    show_notification = true;
    superAdminPubkey = pubkey;
    pubkey = "";
    name = "";
    await getAdmins();
  }

  async function addUser() {
    const result = await add_user(pubkey, Number(selected_role), name);
    const parsedResult = JSON.parse(result);
    success = parsedResult.success || false;
    message = parsedResult.message;
    show_notification = true;
    pubkey = "";
    selected_role = "1";
    name = "";
    await getAdmins();
  }

  async function handleSubmit() {
    loading = true;
    if (!superAdminExist) {
      await setSuperAdmin();
    } else {
      await addUser();
    }
    loading = false;
  }

  async function checkSuperAdminExist() {
    const result = await get_super_admin();
    const parsedResult = JSON.parse(result);
    if (
      parsedResult?.success &&
      parsedResult.message === "super admin record"
    ) {
      superAdminExist = true;
      superAdminPubkey = parsedResult.data.pubkey;
    }
  }

  async function getAdmins() {
    const result = await list_admins();
    const parsedResult = JSON.parse(result);
    if (parsedResult.success) {
      const newAdmin = [];
      for (let i = 0; i < parsedResult.data.length; i++) {
        const admin = parsedResult.data[i];
        newAdmin.push({
          id: admin.pubkey,
          pubkey: shortPubkey(admin.pubkey),
          role: formatRoles(admin.role),
          name: formatUsername(admin.name) || "",
        });
      }
      admins = [...newAdmin];
    }
  }

  async function deleteSubAdmin(pubkey) {
    const result = await delete_sub_admin(pubkey);
    const parsedResult = JSON.parse(result);
    if (parsedResult.success) {
      if (parsedResult.message === "sub admin deleted successfully") {
        success = true;
      } else {
        success = false;
      }
      message = parsedResult.message;
      show_notification = true;
      await getAdmins();
    }
  }

  function toggleAdmin() {
    superAdminExist = !superAdminExist;
  }

  function formatRoles(role) {
    if (role === "admin") {
      return "Admin";
    } else if (role === "sub_admin") {
      return "Sub Admin";
    } else if (role === "member") {
      return "Member";
    } else {
      return role;
    }
  }

  function formatUsername(name) {
    if (!name) {
      return "";
    }
    if (name.length <= 20) {
      return name;
    }
    return `${name.substring(0, 16)}...`;
  }

  onMount(async () => {
    await checkSuperAdminExist();
    await getAdmins();
  });
</script>

<div class="nav-wrapper">
  <div class="super-admin-container">
    {#if superAdminPubkey}
      <div class="update_super_admin_container">
        <button class="update_super_admin_btn" on:click={toggleAdmin}>
          {#if !superAdminExist && superAdminPubkey}
            Add User
          {:else}
            Update Super Admin pubkey
          {/if}</button
        >
      </div>
    {/if}
    {#if show_notification}
      <InlineNotification
        lowContrast
        kind={success ? "success" : "error"}
        title={success ? "Success:" : "Error:"}
        subtitle={message}
        timeout={3000}
        on:close={(e) => {
          e.preventDefault();
          show_notification = false;
        }}
      />
    {/if}
    <TextInput
      labelText={`${
        superAdminExist ? "Add User Pubkey" : "Super Admin Pubkey"
      }`}
      placeholder={`${
        superAdminExist ? "Enter user pubkey..." : "Enter super admin pubkey..."
      }`}
      bind:value={pubkey}
    />
    {#if superAdminExist}
      <Dropdown
        titleText="Primary contact"
        bind:selectedId={selected_role}
        {items}
      />
    {/if}
    <div class="name_input">
      <TextInput
        labelText={`Name (Optional)`}
        placeholder={`Enter name`}
        bind:value={name}
      />
    </div>
    <div class="set-super-admin-btn-container">
      <Button
        on:click={handleSubmit}
        disabled={!pubkey ||
          loading ||
          (superAdminExist && selected_role === "1")}
      >
        {#if loading}
          <InlineLoading />
        {:else}
          Submit
        {/if}
      </Button>
    </div>
  </div>
  <div class="data_table_container">
    {#if superAdminExist}
      <DataTable
        zebra
        headers={[
          { key: "pubkey", value: "Public Key" },
          { key: "role", value: "Role" },
          { key: "name", value: "Name" },
          { key: "action", value: "" },
        ]}
        rows={admins}
      >
        <svelte:fragment slot="cell" let:row let:cell>
          {#if cell.key === "action"}
            {#if row.role !== "Admin"}
              <button
                on:click={() => deleteSubAdmin(row.id)}
                class="deleteButton">Delete</button
              >
            {/if}
          {:else}
            {cell.value}
          {/if}
        </svelte:fragment>
      </DataTable>
    {/if}
  </div>
</div>

<style>
  .nav-wrapper {
    padding: 0px 25px;
  }
  .super-admin-container {
    display: flex;
    flex-direction: column;
    margin-top: 1.5rem;
  }

  .set-super-admin-btn-container {
    margin-top: 0.5rem;
  }

  .update_super_admin_container {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 2rem;
  }

  .update_super_admin_btn {
    border: none;
    padding: 0.7rem 1rem;
    border-radius: 0.3rem;
    cursor: pointer;
  }

  .update_super_admin_btn:hover {
    background-color: #f5f5dc;
  }

  .data_table_container {
    margin-top: 1.5rem;
  }

  .deleteButton {
    /* background-color: #313342; */
    background-color: #161616;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    color: white;
    border: none;
    cursor: pointer;
  }

  .name_input {
    margin-top: 1rem;
  }
</style>
