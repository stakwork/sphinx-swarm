<script lang="ts">
  import { onMount } from "svelte";
  import {
    add_boltwall_admin_pubkey,
    add_user,
    delete_sub_admin,
    list_admins,
    update_admin_pubkey,
    update_user,
  } from "../../../api/swarm";
  import { shortPubkey } from "../../../helpers";
  import Modal from "../../modal.svelte";
  import Input from "../../input/input.svelte";
  import Select from "../../select/select.svelte";
  import { ToastNotification } from "carbon-components-svelte";
  import { activeUser, boltwallSuperAdminPubkey } from "../../../store";

  let users: {
    id: string;
    name: string;
    pubkey: string;
    role: string;
    identifier: number;
  }[] = [];
  let currentUser: {
    id: string;
    name: string;
    pubkey: string;
    role: string;
    identifier: number;
  } = { id: "", pubkey: "", name: "", role: "", identifier: 0 };

  $: openAddUserModel = false;
  $: openEditAdmin = false;
  $: openDeleteUser = false;
  $: openEditUserModal = false;

  let userpubkey = "";
  let adminpubkey = "";
  let username = "";
  let editUsername = "";
  let editPubkey = "";
  let editRole = "";
  let superAdminUsername = "";
  let role = "1";
  $: success = false;
  $: message = "";
  $: show_notification = false;
  $: addUserSuccess = false;
  $: is_admin_Loading = false;
  $: isDeleteUserLoading = false;
  $: allowUserEdit = false;
  $: is_edit_Loading = false;

  const roles = [
    { value: "1", label: "Select Role" },
    { value: "2", label: "Admin" },
    { value: "3", label: "Member" },
  ];

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
    console.log(parsedResult);
    if (parsedResult.success) {
      const newAdmin = [];
      for (let i = 0; i < parsedResult.data.length; i++) {
        const admin = parsedResult.data[i];
        newAdmin.push({
          id: admin.pubkey,
          pubkey: shortPubkey(admin.pubkey),
          role: formatRoles(admin.role),
          name: formatUsername(admin.name) || "",
          identifier: admin.id,
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

  function openEditAdminModal() {
    openEditAdmin = true;
  }

  function closeEditAdminModal() {
    openEditAdmin = false;
  }

  function openDeleteUserHandler() {
    openDeleteUser = true;
  }

  function fromEditToDeleteHandler() {
    closeDeleteUserHandler();
    editUserHandler(currentUser.id);
  }

  function closeDeleteUserHandler() {
    openDeleteUser = false;
  }

  function closeEditUserHandler() {
    openEditUserModal = false;
    clearAllEdit();
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

  function updateEditUsername(value) {
    editUsername = value;
    checkIsEdit();
  }

  function updateEditPubkey(value) {
    editPubkey = value;
    checkIsEdit();
  }

  function updateRoleChange(value) {
    role = value;
  }

  function updateEditRoleChange(value) {
    editRole = value;
    checkIsEdit();
  }

  function clearAllEdit() {
    editPubkey = "";
    editUsername = "";
    editRole = "";
    allowUserEdit = false;
  }

  function checkIsEdit() {
    const role = findRoleByLabel(currentUser.role);
    if (editRole === "1") {
      allowUserEdit = false;
    } else if (
      currentUser.id !== editPubkey ||
      currentUser.name !== editUsername ||
      role.value !== editRole
    ) {
      allowUserEdit = true;
    } else {
      allowUserEdit = false;
    }
  }

  function updateAdminPubkey(value) {
    adminpubkey = value;
  }

  function updateAdminUsername(value) {
    superAdminUsername = value;
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

  function findUser(pubkey: string) {
    for (let i = 0; i < users.length; i++) {
      const user = users[i];
      if (user.id === pubkey) {
        currentUser = { ...user };
        return;
      }
    }
  }

  async function editAdminSaveHandler() {
    is_admin_Loading = true;
    try {
      const result = await add_boltwall_admin_pubkey(
        adminpubkey,
        superAdminUsername
      );
      const parsedResult = JSON.parse(result);
      const swarmAdmin = await update_admin_pubkey(adminpubkey, $activeUser);

      boltwallSuperAdminPubkey.set(adminpubkey);
      is_admin_Loading = false;
      if (parsedResult.success) {
        adminpubkey = "";
        message = "Super Admin Updated Successfully";
        await getAdmins();
        handleAddUserSuccess();
        closeEditAdminModal();
      }
    } catch (error) {
      is_admin_Loading = false;
      //TODO:: Handle error properly
      console.log(
        `ERROR SETTING BOLTWALL SUPER ADMIN: ${JSON.stringify(error)}`
      );
    }
  }

  function findRoleByLabel(label: string) {
    for (let i = 0; i < roles.length; i++) {
      if (roles[i].label === label) {
        return roles[i];
      }
    }
  }

  async function editAdminHandler(pubkey: string) {
    findUser(pubkey);
    superAdminUsername = currentUser.name;
    adminpubkey = currentUser.id;
    openEditAdminModal();
  }

  function deleteUserHandler(pubkey: string) {
    findUser(pubkey);
    openEditUserModal = false;
    openDeleteUserHandler();
  }

  function editUserHandler(pubkey: string) {
    findUser(pubkey);
    if (!editUsername) editUsername = currentUser.name;
    if (!editPubkey) editPubkey = currentUser.id;
    if (!editRole) {
      const role = findRoleByLabel(currentUser.role);
      editRole = role.value;
    }
    openEditUserModal = true;
  }

  async function handleEditUser() {
    //set loading state
    is_edit_Loading = true;

    try {
      //send result to boltwall
      const result = await update_user({
        pubkey: editPubkey,
        name: editUsername,
        role: Number(editRole),
        id: Number(currentUser.identifier),
      });

      const parsedResult = JSON.parse(result);
      success = parsedResult.success || false;
      message = parsedResult.message;
      if (success) {
        //get all admins again
        await getAdmins();

        handleAddUserSuccess();
        // stop loading
        is_edit_Loading = false;

        //close modal
        closeEditUserHandler();
      } else {
        show_notification = true;
      }
    } catch (error) {
      is_edit_Loading = false;
      //TODO:: Handle error properly
      console.log(`ERROR UPDATING USER: ${JSON.stringify(error)}`);
    }
  }

  async function deleteUser() {
    isDeleteUserLoading = true;
    try {
      const result = await delete_sub_admin(currentUser.id);
      const parsedResult = JSON.parse(result);
      isDeleteUserLoading = false;
      if (parsedResult.success) {
        message = "User Deleted Successfully";
        await getAdmins();
        handleAddUserSuccess();
        closeDeleteUserHandler();
        clearAllEdit();
      }
    } catch (error) {
      isDeleteUserLoading = false;
      console.log(`ERROR DELETING USER: ${JSON.stringify(error)}`);
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
            <td class="column_pubkey table_column">
              {user.pubkey}
              <div class="tool_tip_container">{user.id}</div>
            </td>
            <td class="column_role table_column">{user.role}</td>
            <td class="column_action table_column"
              >{#if user.role === "Super Admin"}
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <img
                  on:click={() => editAdminHandler(user.id)}
                  src="swarm/edit.svg"
                  alt="edit"
                  class="action_icon"
                />
              {:else}
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <img
                  src="swarm/edit.svg"
                  alt="edit"
                  class="action_icon"
                  on:click={() => editUserHandler(user.id)}
                />
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
              options={roles}
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
  <Modal isOpen={openEditAdmin} onClose={closeEditAdminModal}>
    <div class="edit_admin_container">
      <div class="admin_image_container">
        <img src="swarm/admin.svg" alt="admin" />
      </div>
      <h3 class="edit_admin_text">Edit Admin</h3>
      <div class="edit_admin_form_container">
        <Input
          label="Username"
          placeholder="Type Username Here"
          onInput={updateAdminUsername}
          value={superAdminUsername}
        />
        <Input
          label="Pubkey"
          placeholder="Type Pubkey Here"
          onInput={updateAdminPubkey}
          value={adminpubkey}
        />
        <div class="edit_admin_btn_container">
          <button on:click={closeEditAdminModal} class="edit_admin_cancel_btn"
            >Cancel</button
          >
          <button
            disabled={is_admin_Loading || !adminpubkey}
            class="edit_admin_save_btn"
            on:click={editAdminSaveHandler}
          >
            {#if is_admin_Loading === true}
              <div class="loading-spinner"></div>
            {:else}
              Save Changes
            {/if}
          </button>
        </div>
      </div>
    </div>
  </Modal>
  <Modal isOpen={openDeleteUser} onClose={closeDeleteUserHandler}>
    <div class="delete_user_container">
      <div class="user_details_container">
        <div class="user_image_container">
          <img src="swarm/user.svg" alt="user" />
        </div>
        <p>{currentUser.name}</p>
      </div>
      <p class="delete_warning_text">
        Are you sure you want to <span class="delete_warning_text_emphasis"
          >Delete this user?</span
        >
      </p>
      <div class="delete_button_container">
        <button
          class="delete_user_cancel_btn"
          on:click={fromEditToDeleteHandler}>Cancel</button
        >
        <button
          class="delete_user_btn"
          disabled={isDeleteUserLoading}
          on:click={deleteUser}
        >
          {#if isDeleteUserLoading === true}
            <div class="delete_loading-spinner"></div>
          {:else}
            Delete
          {/if}
        </button>
      </div>
    </div>
  </Modal>
  <Modal isOpen={openEditUserModal} onClose={closeEditUserHandler}>
    <div class="edit_user_container">
      <div class="close_container">
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <img
          src="swarm/close.svg"
          alt="close"
          class="close_icon"
          on:click={closeEditUserHandler}
        />
      </div>
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
      <div class="add_user_body">
        <h3 class="add_user_heading">Edit User</h3>
        <div class="input_container">
          <Input
            label="Name"
            placeholder="Enter Name ..."
            onInput={updateEditUsername}
            value={editUsername}
          />
          <Input
            label="Pubkey"
            placeholder="Paste Pubkey  ..."
            onInput={updateEditPubkey}
            value={editPubkey}
          />
          <Select
            value={editRole}
            options={roles}
            label="Select Role"
            valueChange={updateEditRoleChange}
          />
        </div>
        <div class="edit_user_btn_container">
          <button
            on:click={() => deleteUserHandler(currentUser.id)}
            class="delete_btn">Delete</button
          >
          <button
            on:click={handleEditUser}
            class="save_changes_btn"
            disabled={!allowUserEdit || is_edit_Loading}>Save Changes</button
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
    text-transform: capitalize;
  }

  .column_pubkey {
    color: #909baa;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.5rem; /* 171.429% */
    cursor: pointer;
    position: relative;
  }

  .column_pubkey:hover .tool_tip_container {
    visibility: visible;
    opacity: 1;
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

  .edit_admin_container {
    padding: 2.37rem 2.13rem 3.94rem 2.13rem;
    display: flex;
    align-items: center;
    flex-direction: column;
    justify-content: center;
  }

  .admin_image_container {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 1.63rem;
  }

  .admin_image_container img {
    width: 2.5rem;
    height: 2.5rem;
  }

  .edit_admin_text {
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 1.875rem;
    font-style: normal;
    font-weight: 700;
    line-height: 1rem; /* 53.333% */
    letter-spacing: 0.01875rem;
  }

  .edit_admin_form_container {
    display: flex;
    flex-direction: column;
    width: 15.625rem;
    margin-top: 1.94rem;
    gap: 1rem;
  }

  .edit_admin_btn_container {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1.31rem;
  }

  .edit_admin_cancel_btn {
    display: flex;
    width: 7.5rem;
    height: 2.5rem;
    padding: 0.75rem 1rem;
    justify-content: center;
    align-items: center;
    border-radius: 0.375rem;
    border: 1px solid rgba(107, 122, 141, 0.5);
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 135.714% */
    background-color: transparent;
    cursor: pointer;
  }

  .edit_admin_save_btn {
    display: flex;
    width: 7.5rem;
    height: 2.5rem;
    padding: 0.75rem 1rem;
    justify-content: center;
    align-items: center;
    border-radius: 0.375rem;
    background: #618aff;
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 135.714% */
    border: none;
    cursor: pointer;
  }

  .edit_admin_save_btn:disabled {
    cursor: not-allowed;
  }

  .loading-spinner {
    border: 2px solid #fff;
    border-top: 2px solid #618aff;
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

  .delete_user_container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2.06rem 2.19rem 3rem 2.19rem;
  }

  .user_details_container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .user_details_container img {
    width: 2.5rem;
    height: 2.5rem;
  }

  .user_details_container p {
    color: #6b7a8d;
    text-align: center;
    font-family: "Barlow";
    font-size: 1.125rem;
    font-style: normal;
    font-weight: 500;
    line-height: 1.5rem; /* 133.333% */
    text-transform: capitalize;
  }

  .delete_warning_text {
    width: 15.5625rem;
    color: var(--Primary-Text, #fff);
    text-align: center;
    font-family: "Barlow";
    font-size: 1.25rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.75rem; /* 140% */
    margin-top: 2.19rem;
    margin-bottom: 3.25rem;
  }

  .delete_warning_text_emphasis {
    font-weight: 700;
  }

  .delete_button_container {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 15.5625rem;
  }

  .delete_user_cancel_btn {
    display: flex;
    width: 7rem;
    height: 2.5rem;
    padding: 0.75rem 1rem;
    justify-content: center;
    align-items: center;
    border-radius: 0.375rem;
    border: 1px solid rgba(107, 122, 141, 0.5);
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 135.714% */
    background-color: transparent;
    cursor: pointer;
  }

  .delete_user_btn {
    display: flex;
    width: 7rem;
    height: 2.5rem;
    padding: 0.75rem;
    justify-content: center;
    align-items: center;
    border-radius: 0.375rem;
    background: #ed7474;
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 600;
    line-height: 0rem; /* 0% */
    letter-spacing: 0.00875rem;
    border: none;
    cursor: pointer;
  }

  .delete_user_btn:disabled {
    cursor: not-allowed;
  }

  .delete_loading-spinner {
    border: 2px solid #fff; /* Light grey */
    border-top: 2px solid #ed7474; /* Blue */
    border-radius: 50%;
    width: 1.125rem;
    height: 1.125rem;
    animation: spin 1s linear infinite;
  }

  .tool_tip_container {
    visibility: hidden;
    position: absolute;
    border-radius: 0.25rem;
    padding: 0.625rem;
    background: #000;
    width: 19.6875rem;
    overflow-wrap: break-word;
    text-align: center;
    z-index: 1;
    cursor: default;
    font-family: "Barlow";
    font-size: 0.75rem;
    color: #fff;
    opacity: 0;
  }

  .edit_user_container {
    display: flex;
    flex-direction: column;
    width: 19.875rem;
  }

  .edit_user_btn_container {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 2rem;
  }

  .delete_btn {
    background-color: #ed74741a;
    height: 2.5rem;
    width: 4.875rem;
    border-radius: 0.375rem;
    padding: 0.75rem;
    color: #ed7474;
    font-weight: 600;
    font-size: 0.875rem;
    font-family: "Barlow";
    border: none;
    cursor: pointer;
  }

  .delete_btn:hover {
    background-color: #ed747426;
  }

  .save_changes_btn {
    width: 9.375rem;
    height: 2.5rem;
    border-radius: 0.375rem;
    padding: 0.75rem;
    background-color: #618aff;
    border: none;
    color: #ffffff;
    font-weight: 600;
    font-size: 0.875rem;
    font-family: "Barlow";
    cursor: pointer;
  }

  .save_changes_btn:hover {
    background-color: #5078f2;
  }

  .save_changes_btn:disabled {
    background-color: #30334280;
    color: #52566e;
    cursor: not-allowed;
  }
</style>
