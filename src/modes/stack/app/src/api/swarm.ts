import { send_cmd } from ".";

export async function get_config() {
  send_cmd("Swarm", { cmd: "GetConfig" });
}
