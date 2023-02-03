import { send_cmd } from "./cmd";
import type { Cmd } from "./cmd";
import { root } from "./cmd";

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

export async function update_password(password, token) {
  const r = await fetch(`${root}/admin/password`, {
    method: "PUT",
    body: JSON.stringify({
      password,
    }),
    headers: {
      "x-jwt": token
    }
  });

  const result = await r.json();
  return result;
}
