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
