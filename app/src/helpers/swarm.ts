import type { Writable } from "svelte/store";
import { get_image_tags } from "../api/swarm";
import type { Stack } from "../nodes";
import { swarm } from "../api";

export async function getVersionFromDigest(
  digest: string,
  org_image_name: string,
  page: string,
  page_size: string
) {
  try {
    const splittedDigest = digest.split("@")[1];
    const response = await get_image_tags(org_image_name, page, page_size);

    const tags = JSON.parse(response);

    for (let i = 0; i < tags.results.length; i++) {
      const result = tags.results[i];
      if (result.digest === splittedDigest) {
        if (result.name !== "latest") {
          return result.name;
        } else {
          const architectureDigests = [];
          for (let j = 0; j < result.images.length; j++) {
            architectureDigests.push(result.images[j].digest);
          }
          return findArchitectureDigest(architectureDigests, tags.results);
        }
      }
    }

    if (tags.next) {
      const urlString = tags.next;
      const url = new URL(urlString);
      const params = new URLSearchParams(url.search);

      const page = params.get("page");
      const page_size = params.get("page_size");

      return await getVersionFromDigest(
        digest,
        org_image_name,
        page,
        page_size
      );
    }
  } catch (error) {
    throw error;
  }
}

function findArchitectureDigest(architectureDigests, results) {
  for (let i = 0; i < results.length; i++) {
    const result = results[i];
    if (result.name !== "latest") {
      for (let j = 0; j < result.images.length; j++) {
        const image = result.images[j];
        if (architectureDigests.includes(image.digest)) {
          return result.name;
        }
      }
    }
  }
}

export async function getImageVersion(
  node_name: string,
  stack: Writable<Stack>
) {
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
  const image_digest_response = await swarm.get_image_digest(
    `sphinxlightning/${image_name}`
  );
  if (image_digest_response.success) {
    const version = await getVersionFromDigest(
      image_digest_response.digest,
      `sphinxlightning/${image_name}`,
      "1",
      "100"
    );

    if (version) {
      stack.update((stack) => {
        for (let i = 0; i < stack.nodes.length; i++) {
          const oldNode = { ...stack.nodes[i] };
          if (oldNode.name === node_name) {
            const newNode = { ...oldNode, version };
            stack.nodes[i] = { ...newNode };
            break;
          }
        }
        return stack;
      });
    }
  }
}
