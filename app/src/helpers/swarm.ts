export async function getVersionFromDigest(digest: string, image_name: string) {
  try {
    const splittedDigest = digest.split("@")[1];

    const response = await fetch(
      `http://hub.docker.com/v2/repositories/sphinxlightning/${image_name}/tags?page_size=100000`
    );
    if (!response.ok) {
      throw new Error("Error fetching image from Docker Hub");
    }

    const tags = await response.json();
    for (let i = 0; i < tags.results.length; i++) {
      const result = tags.results[i];
      if (result.digest === splittedDigest && result.name !== "latest") {
        return result.name;
      }
    }
  } catch (error) {
    throw error;
  }
}
