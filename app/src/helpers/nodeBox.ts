export function determineBorderColor({
  is_latest,
  version,
  latest_version,
}: {
  is_latest?: boolean;
  version?: string;
  latest_version?: string;
}) {
  if (version && latest_version && is_latest) {
    return "node-uptodate-border";
  }

  if (
    latest_version &&
    latest_version !== "unavailable" &&
    version !== "unavailable" &&
    !is_latest
  ) {
    return "node-outdated-border";
  }

  return "node-internal";
}

export function determineIfShouldUpdate({
  is_latest,
  version,
  latest_version,
}: {
  is_latest?: boolean;
  version?: string;
  latest_version?: string;
}) {
  if (
    latest_version &&
    latest_version !== "unavailable" &&
    version !== "unavailable" &&
    !is_latest
  ) {
    return true;
  }
  return false;
}
