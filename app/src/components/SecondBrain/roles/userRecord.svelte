<script lang="ts">
  import { onMount } from "svelte";
  import { list_admins } from "../../../api/swarm";
  import { shortPubkey } from "../../../helpers";

  const tableHeaderColumns = ["Name", "Public Key", "Role", ""];
  let users: { id: string; name: string; pubkey: string; role: string }[] = [];

  const tobi = [
    {
      id: "03a394d0ebf0d003124ab130c6b12b8b990a50a30a464354800a51981ba745bb07",
      pubkey: "03a394d0ebf0d00...",
      role: "Super Admin",
      name: "Alice",
    },

    {
      id: "03a394d0ebf0d003124ab130c6b12b8b990a50a30a464354800a51981ba745bb07",
      pubkey: "03a394d0ebf0d00...",
      role: "Admin",
      name: "Jonathan",
    },
    {
      id: "03a394d0ebf0d003124ab130c6b12b8b990a50a30a464354800a51981ba745bb07",
      pubkey: "03a394d0ebf0d00...",
      role: "Member",
      name: "Jonathan",
    },
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
      console.log(users);
    }
  }

  async function determineHeaderClass(index: number, arrayLength: number) {
    if (index === 0) {
      return "leftHeaderColumn";
    } else if (index === arrayLength - 1) {
      return "rightHeaderColumn";
    }
  }

  onMount(async () => {
    //Get All users
    await getAdmins();
  });
</script>

<div class="container">
  <div class="header_container">
    <h2 class="heading_text">Roles</h2>
    <button class="add_user_btn">Add User</button>
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
        {#each tobi as user}
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
</style>
