import type { ILightningBot } from "../types/types";
import type { Writable } from "svelte/store";
import { get } from "svelte/store";
import { get_lightning_bots_detail } from "../../../../../../app/src/api/swarm";
import { remotes } from "../store";

export function splitHost(hostFullPath: string) {
  if (hostFullPath) {
    const arr = hostFullPath.split(".");
    if (arr[0]) {
      return arr[0];
    }
    return "";
  }
  return "";
}

export function getSwarmNumber(default_host: string) {
  // Regular expression to match the number in the string
  const match = default_host.match(/\d+/);

  if (match) {
    return match[0];
  } else {
    return "";
  }
}

export function isValidVanityAddress(vanity_address: string) {
  const valid_chars = /^[a-zA-Z0-9-]+$/; // Only letters, numbers, and hyphens
  const consecutive_hyphens = /--/; // Check for consecutive hyphens

  if (vanity_address.startsWith("-") || vanity_address.endsWith("-")) {
    return "Hyphen cannot be the first or last character.";
  }

  if (consecutive_hyphens.test(vanity_address)) {
    return "Hyphens cannot appear consecutively.";
  }

  if (!valid_chars.test(vanity_address) && vanity_address) {
    return "Vanity address can only contain letters, numbers, and hyphens.";
  }

  return "";
}

export function extract_instance_type(instance_type: string) {
  const split_array = instance_type.split(" ");
  const temp_instance = split_array[1];
  return temp_instance.slice(1, temp_instance.length - 1);
}

export async function fectAndRefreshLightningBotDetails(
  lightningBots: Writable<ILightningBot[]>
): Promise<{ success: boolean; message: string }> {
  try {
    let res = await get_lightning_bots_detail();
    let message = res.message;
    if (res.success) {
      if (res.data) {
        lightningBots.set(res.data);
      }
      return { success: true, message };
    } else {
      return { success: false, message };
    }
  } catch (error) {
    console.log("error: ", error);
    return {
      success: false,
      message: "Error occured while trying to get lightning bots details",
    };
  }
}

export function getRemoteByHost(host: string) {
  const swarms = get(remotes);
  for (let i = 0; i < swarms.length; i++) {
    const remote = swarms[i];
    if (remote.host === host) {
      return remote;
    }
  }
  return null;
}
