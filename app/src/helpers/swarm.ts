import type { Writable } from "svelte/store";
import {
  get_all_image_actual_version,
  get_image_tags,
  get_lightning_peers,
} from "../api/swarm";
import type { Stack, Node } from "../nodes";
import { lightningPeers, swarmVersion } from "../store";

export async function getImageVersion(
  stack: Writable<Stack>,
  selectedNode: Writable<Node>
) {
  const image_versions = await get_all_image_actual_version();
  if (image_versions.success) {
    let version_object = {};

    for (let i = 0; i < image_versions.data.length; i++) {
      const image_data = image_versions.data[i];
      version_object[image_data.name] = { ...image_data };
    }

    swarmVersion.set(version_object["swarm"]);

    stack.update((stack) => {
      for (let i = 0; i < stack.nodes.length; i++) {
        const version_details = version_object[stack.nodes[i].name];
        const newNode = {
          ...stack.nodes[i],
          ...(stack.nodes[i].name !== "neo4j" && {
            version: version_details.version,
            latest_version: version_details.latest_version,
            is_latest: version_details.is_latest,
          }),
        };

        selectedNode.update((node) =>
          node && node.name === newNode.name ? { ...newNode } : node
        );

        stack.nodes[i] = { ...newNode };
      }

      return stack;
    });
  }
}

export async function handleGetImageTags(node_name: string): Promise<string[]> {
  let image_name = `sphinx-${node_name}`;
  if (node_name === "relay") {
    image_name = `sphinx-relay-swarm`;
  } else if (node_name === "cln") {
    image_name = `cln-sphinx`;
  } else if (node_name === "navfiber") {
    image_name = `sphinx-nav-fiber`;
  } else if (node_name === "cache") {
    image_name = ``;
  } else if (node_name === "jarvis") {
    image_name = `sphinx-jarvis-backend`;
  }

  const response = await get_image_tags(
    `sphinxlightning/${image_name}`,
    "1",
    "100"
  );

  const tags = [];

  try {
    const parsedRes = JSON.parse(response);
    for (let i = 0; i < parsedRes.results.length; i++) {
      const image = parsedRes.results[i];
      tags.push(image.name);
      if (tags.length === 10) {
        return tags;
      }
    }
    return tags;
  } catch (error) {
    console.log(error);
    return [];
  }
}

export function splitPubkey(pubkey: string) {
  if (pubkey.includes("_")) {
    return pubkey.split("_")[0];
  }
  if (pubkey.includes(":")) {
    return pubkey.split(":")[0];
  }
  return pubkey;
}

export async function handleGetLightningPeers() {
  const res = await get_lightning_peers();
  if (Array.isArray(res)) {
    lightningPeers.set(res);
  }
}

export function formatPubkey(pk: string) {
  return `${pk.substring(0, 6)}...${pk.substring(pk.length - 6)}`;
}

export function formatPubkeyAliasDisplay(pubkey: string, alias: string) {
  return `${alias} (${formatPubkey(pubkey)})`;
}
