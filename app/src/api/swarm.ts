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

export async function get_image_digest(image_name: string) {
  return await swarmCmd("GetImageDigest", image_name);
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

export async function add_boltwall_admin_pubkey(pubkey: string, name?: string) {
  return await swarmCmd("AddBoltwallAdminPubkey", { pubkey, name });
}

export async function get_super_admin() {
  return await swarmCmd("GetBoltwallSuperAdmin");
}

export async function add_user(pubkey: string, role: number, name?: string) {
  return await swarmCmd("AddBoltwallUser", { pubkey, role, name });
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

export async function update_swarm() {
  return await swarmCmd("UpdateSwarm");
}

export async function update_graph_accessibility(status: boolean) {
  return await swarmCmd("UpdateBoltwallAccessibility", status);
}

export async function get_graph_accessibility() {
  return await swarmCmd("GetBoltwallAccessibility");
}

export async function get_second_brain_about_details() {
  return await swarmCmd("GetSecondBrainAboutDetails");
}

export async function get_feature_flag() {
  return await swarmCmd("GetFeatureFlags");
}

export async function update_second_brain_about(data) {
  return await swarmCmd("UpdateSecondBrainAbout", data);
}

export async function update_feature_flags(data: { [key: string]: boolean }) {
  return await swarmCmd("UpdateFeatureFlags", data);
}

export async function get_api_token() {
  return await swarmCmd("GetApiToken");
}

export async function add_new_swarm(new_swarm: {
  host: string;
  instance: string;
  description: string;
}) {
  return await swarmCmd("AddNewSwarm", { ...new_swarm });
}

export async function update_swarm_details(swarm_info: {
  id: string;
  host: string;
  description: string;
  instance: string;
}) {
  return await swarmCmd("UpdateSwarm", { ...swarm_info });
}

export async function delete_swarm(data: { host: string }) {
  return await swarmCmd("DeleteSwarm", { ...data });
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

export async function update_admin_pubkey(pubkey, token) {
  const r = await fetch(`${root}/admin/pubkey`, {
    method: "PUT",
    body: JSON.stringify({
      pubkey,
    }),
    headers: {
      "x-jwt": token,
    },
  });

  const result = await r.json();
  return result;
}

export async function get_challenge() {
  const r = await fetch(`${root}/challenge`);
  const result = await r.json();
  return result;
}

export async function get_challenge_status(challenge) {
  const r = await fetch(`${root}/poll/${challenge}`);
  const result = await r.json();
  return result;
}

export async function get_signup_challenge_status(challenge, username, token) {
  const r = await fetch(
    `${root}/poll_signup_challenge/${challenge}?username=${username}`,
    {
      headers: {
        "x-jwt": token,
      },
    }
  );
  const result = await r.json();
  return result;
}

export async function get_signup_challenge(token) {
  const r = await fetch(`${root}/signup_challenge`, {
    headers: { "x-jwt": token },
  });
  const result = await r.json();
  return result;
}

export async function get_image_tags(
  org_image_name: string,
  page: string,
  page_size: string
) {
  return await swarmCmd("GetDockerImageTags", {
    page,
    page_size,
    org_image_name,
  });
}

export async function update_user({
  pubkey,
  name,
  role,
  id,
}: {
  pubkey: string;
  name: string;
  role: number;
  id: number;
}) {
  return await swarmCmd("UpdateUser", { pubkey, name, role, id });
}

export async function get_child_swarm_config({ host }: { host: string }) {
  return await swarmCmd("GetChildSwarmConfig", { host });
}

export async function get_child_swarm_containers({ host }: { host: string }) {
  return await swarmCmd("GetChildSwarmContainers", { host });
}

export async function stop_child_swarm_containers({
  nodes,
  host,
}: {
  nodes: string[];
  host: string;
}) {
  return await swarmCmd("StopChildSwarmContainers", { nodes, host });
}

export async function start_child_swarm_containers({
  nodes,
  host,
}: {
  nodes: string[];
  host: string;
}) {
  return await swarmCmd("StartChildSwarmContainers", { nodes, host });
}

export async function update_child_swarm_containers({
  nodes,
  host,
}: {
  nodes: string[];
  host: string;
}) {
  return await swarmCmd("UpdateChildSwarmContainers", { nodes, host });
}

export async function create_new_swarm_ec2({
  name,
  vanity_address,
}: {
  vanity_address?: string;
  name: string;
}) {
  return await swarmCmd("CreateNewEc2Instance", {
    vanity_address,
    name,
  });
}
