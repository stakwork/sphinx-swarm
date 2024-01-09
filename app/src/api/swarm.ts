import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";
import { root } from "./cmd";
export interface Container {
  Command: string;
  Created: number;
  Id: string;
  Image: string;
  ImageID: string;
  State: string;
  Status: string;
  Names: string[];
  [key: string]: any;
}

async function swarmCmd(cmd: Cmd, content?: any) {
  return await send_cmd("Swarm", { cmd, content });
}

export async function get_config() {
  return await swarmCmd("GetConfig");
}

export async function get_logs(name) {
  return await swarmCmd("GetContainerLogs", name);
}

export async function get_node_images(name, page) {
  return await swarmCmd("ListVersions", { name, page });
}

export async function update_node_instance(name, version) {
  return await swarmCmd("UpdateInstance", { name, version });
}

export async function list_containers() {
  return await swarmCmd("ListContainers");
}

export async function stop_container(id: string) {
  return await swarmCmd("StopContainer", id);
}

export async function start_container(id: string) {
  return await swarmCmd("StartContainer", id);
}

export async function update_node(id: string) {
  return await swarmCmd("UpdateNode", { id, version: "latest" });
}

export async function get_container_stat(name?: string) {
  return await swarmCmd("GetStatistics", name);
}

export async function add_boltwall_admin_pubkey(pk: string) {
  return await swarmCmd("AddBoltwallAdminPubkey", pk);
}

export async function get_super_admin() {
  return await swarmCmd("GetBoltwallSuperAdmin");
}

export async function add_boltwall_sub_admin_pubkey(pk: string) {
  return await swarmCmd("AddBoltwallSubAdminPubkey", pk);
}

export async function list_admins() {
  return await swarmCmd("ListAdmins");
}

export async function delete_sub_admin(pk: string) {
  return await swarmCmd("DeleteSubAdmin", pk);
}

export async function list_all_paid_endpoint() {
  return await swarmCmd("ListPaidEndpoint");
}

export async function update_paid_endpoint(id: number, status: boolean) {
  return await swarmCmd("UpdatePaidEndpoint", { id, status });
}

export async function login(username, password) {
  const r = await fetch(`${root}/login`, {
    method: "POST",
    body: JSON.stringify({
      username,
      password,
    }),
  });

  const result = await r.json();
  return result;
}

export async function update_password(password, old_pass, token) {
  const r = await fetch(`${root}/admin/password`, {
    method: "PUT",
    body: JSON.stringify({
      old_pass,
      password,
    }),
    headers: {
      "x-jwt": token,
    },
  });

  const result = await r.json();
  return result;
}

export async function refresh_token(token) {
  const r = await fetch(`${root}/refresh_jwt`, {
    headers: {
      "x-jwt": token,
    },
  });

  const result = await r.json();
  return result;
}
