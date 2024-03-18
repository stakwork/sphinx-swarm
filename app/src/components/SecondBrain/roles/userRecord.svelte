<script lang="ts">
  import { onMount } from "svelte";
  import { add_user, list_admins } from "../../../api/swarm";
  import { shortPubkey } from "../../../helpers";
  import Modal from "../../modal.svelte";
  import Input from "../../input/input.svelte";
  import Select from "../../select/select.svelte";
  import { ToastNotification } from "carbon-components-svelte";

  let users: { id: string; name: string; pubkey: string; role: string }[] = [];

  $: openAddUserModel = false;

  let userpubkey = "";
  let username = "";
  let role = "1";
  $: success = false;
  $: message = "";
  $: show_notification = false;
  $: addUserSuccess = false;

  function formatRoles(role) {
    if (role === "admin") {
      return "Super Admin";
    } else if (role === "sub_admin") {
      return "Admin";
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
      users = [...newAdmin];
    }
  }

  function closeAddUserModal() {
    openAddUserModel = false;
  }

  function openAddUserModal() {
    openAddUserModel = true;
  }

  onMount(async () => {
    //Get All users
    await getAdmins();
  });

  function updateUserPubkey(value) {
    userpubkey = value;
  }

  function updateUserName(value) {
    username = value;
  }

  function updateRoleChange(value) {
    role = value;
  }

  function handleAddUserSuccess() {
    addUserSuccess = true;
    setTimeout(() => {
      addUserSuccess = false;
    }, 3000);
  }

  async function handleCreateUser() {
    const result = await add_user(userpubkey, Number(role), username);
    const parsedResult = JSON.parse(result);
    success = parsedResult.success || false;
    message =
      parsedResult.message === "user added successfully"
        ? "User Added"
        : parsedResult.message;
    if (success) {
      await getAdmins();
      closeAddUserModal();
      userpubkey = "";
      role = "1";
      username = "";
      handleAddUserSuccess();
    } else {
      show_notification = true;
    }
  }
</script>

<div class="container">
  <div class="header_container">
    <h2 class="heading_text">Roles</h2>
    {#if addUserSuccess}
      <div class="add_user_success_info">
        <img src="swarm/check_circle.svg" alt="success" />
        <p>{message}</p>
      </div>
    {:else}
      <button class="add_user_btn" on:click={openAddUserModal}>Add User</button>
    {/if}
  </div>
  <div class="table_container">
    <table class="table">
      <thead>
        <tr class="header_row">
          <th class="column_header leftHeaderColumn">Name</th>
          <th class="column_header">Public Key</th>
          <th class="column_header">Role</th>
          <th class="column_header rightHeaderColumn"></th>
        </tr>
      </thead>
      <tbody>
        {#each users as user}
          <tr class="table_row">
            <td class="column_name table_column">{user.name}</td>
            <td class="column_pubkey table_column">{user.pubkey}</td>
            <td class="column_role table_column">{user.role}</td>
            <td class="column_action table_column"
              >{#if user.role === "Super Admin"}
                <img src="swarm/edit.svg" alt="edit" class="action_icon" />
              {:else}
                <img src="swarm/delete.svg" alt="delete" class="action_icon" />
              {/if}</td
            >
          </tr>
        {/each}
        <tr></tr>
      </tbody>
    </table>
  </div>
  <Modal isOpen={openAddUserModel} onClose={closeAddUserModal}>
    <div class="add_user_container">
      <div class="close_container">
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <img
          src="swarm/close.svg"
          alt="close"
          class="close_icon"
          on:click={closeAddUserModal}
        />
      </div>
      <div class="add_user_body">
        {#if show_notification}
          <div class="toast_container">
            <ToastNotification
              kind={success ? "success" : "error"}
              title={success ? "Success:" : "Error:"}
              subtitle={message}
              timeout={3000}
              on:close={(e) => {
                e.preventDefault();
                show_notification = false;
              }}
              fullWidth={true}
            />
          </div>
        {/if}
        <h3 class="add_user_heading">Add User</h3>
        <div class="form_container">
          <div class="input_container">
            <Input
              label="Name"
              placeholder="Enter Name ..."
              onInput={updateUserName}
              value={username}
            />
            <Input
              label="Pubkey"
              placeholder="Paste Pubkey  ..."
              onInput={updateUserPubkey}
              value={userpubkey}
            />
            <Select
              value={role}
              options={[
                { value: "1", label: "Select Role" },
                { value: "2", label: "Admin" },
                { value: "3", label: "Member" },
              ]}
              label="Select Role"
              valueChange={updateRoleChange}
            />
          </div>
          <button
            disabled={role === "1" || !username || !userpubkey}
            class="add_user_action_btn"
            on:click={handleCreateUser}
            ><img src="swarm/plus.svg" alt="plus" class="plus_sign" />Add User</button
          >
        </div>
      </div>
    </div>
  </Modal>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
  }

  .header_container {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 1.81rem;
    margin-bottom: 1.81rem;
    padding-left: 2.25rem;
    padding-right: 2.25rem;
  }

  .heading_text {
    color: #fff;
    font-family: "Barlow";
    font-size: 1.125rem;
    font-style: normal;
    font-weight: 700;
    line-height: 1rem; /* 88.889% */
    letter-spacing: 0.01125rem;
  }

  .add_user_btn {
    padding: 0.75rem 1rem;
    gap: 0.75rem;
    border-radius: 0.375rem;
    background: #618aff;
    color: var(--White, #fff);
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 135.714% */
    border: none;
    cursor: pointer;
  }

  .table_container {
    width: 100%;
    overflow-y: auto;
    height: 25rem;
  }

  .table {
    width: 100%;
  }

  .column_header {
    color: #6b7a8d;
    font-family: "Barlow";
    font-size: 0.6875rem;
    font-style: normal;
    font-weight: 500;
    line-height: 1.125rem; /* 163.636% */
    letter-spacing: 0.04125rem;
    text-transform: uppercase;
    padding-top: 1.25rem;
    padding-bottom: 1.25rem;
    text-align: left;
    position: sticky;
    top: 0;
    z-index: 1;
    background-color: #23252f;
  }

  .leftHeaderColumn {
    padding-left: 2.25rem;
  }

  .rightHeaderColumn {
    padding-right: 2.25rem;
  }

  .header_row {
    border-bottom: 1px solid rgba(0, 0, 0, 0.25);
  }

  .table_row:nth-child(odd) {
    background: rgba(0, 0, 0, 0.1);
  }

  .table_column {
    padding-top: 0.93rem;
    padding-bottom: 0.93rem;
  }

  .column_name {
    color: #fff;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 500;
    line-height: 1.5rem; /* 171.429% */
    padding-left: 2.25rem;
  }

  .column_pubkey {
    color: #909baa;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.5rem; /* 171.429% */
  }

  .column_role {
    color: #fff;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.5rem; /* 171.429% */
  }

  .column_action {
    padding-right: 2.25rem;
    text-align: right;
  }

  .action_icon {
    width: 1.25rem;
    height: 1.25rem;
    cursor: pointer;
  }

  .add_user_container {
    display: flex;
    flex-direction: column;
    width: 19.875rem;
  }

  .close_container {
    padding-top: 1rem;
    padding-right: 1rem;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .close_icon {
    cursor: pointer;
  }

  .add_user_body {
    display: flex;
    flex-direction: column;
    padding-left: 2.13rem;
    padding-right: 2.13rem;
    padding-top: 0.25rem;
    padding-bottom: 2.5rem;
  }

  .add_user_heading {
    color: #fff;
    font-family: "Barlow";
    font-size: 1.375rem;
    font-style: normal;
    font-weight: 700;
    line-height: 1.125rem; /* 81.818% */
    margin-bottom: 1.13rem;
  }

  .form_container {
    display: flex;
    flex-direction: column;
  }

  .input_container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .add_user_action_btn {
    border-radius: 0.375rem;
    background: #618aff;
    display: flex;
    width: 100%;
    padding: 0.75rem;
    justify-content: center;
    align-items: center;
    gap: 0.375rem;
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 0rem; /* 0% */
    letter-spacing: 0.00875rem;
    outline: none;
    border: none;
    margin-top: 2.5rem;
    cursor: pointer;
  }

  .add_user_action_btn:disabled {
    cursor: not-allowed;
  }

  .plus_sign {
    width: 1.25rem;
    height: 1.25rem;
  }

  .toast_container {
    margin-bottom: 1rem;
  }

  .add_user_success_info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .add_user_success_info p {
    color: #49c998;
    font-family: "Roboto";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 123.077% */
    letter-spacing: 0.00813rem;
    text-transform: capitalize;
  }
</style>
