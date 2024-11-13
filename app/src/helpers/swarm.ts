import type { Writable } from "svelte/store";
import { get_all_image_actual_version } from "../api/swarm";
import type { Stack, Node } from "../nodes";

export async function getImageVersion(
  stack: Writable<Stack>,
  selectedNode: Writable<Node>
) {
  const image_versions = await get_all_image_actual_version();
  if (image_versions.success) {
    let version_object = {};

    for (let i = 0; i < image_versions.data.length; i++) {
      const image_version = image_versions.data[i];
      version_object[image_version.name] = image_version.version;
    }

    stack.update((stack) => {
      for (let i = 0; i < stack.nodes.length; i++) {
        const newNode = {
          ...stack.nodes[i],
          ...(stack.nodes[i].version === "latest" && {
            version: version_object[stack.nodes[i].name],
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
