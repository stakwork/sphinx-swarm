import { get_image_tags } from "../api/swarm";

export async function getVersionFromDigest(digest: string, url: string) {
  try {
    const splittedDigest = digest.split("@")[1];
    const response = await get_image_tags(encodeURIComponent(url));

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
      return await getVersionFromDigest(digest, tags.next);
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
