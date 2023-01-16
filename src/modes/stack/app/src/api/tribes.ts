export interface Person {
  owner_alias: string;
  owner_pubkey: string;
  owner_route_hint: string;
  img: string;
  description: string;
  unique_name: string;
}

export interface Tribe {
  total: number;
  data: TribeData[];
  page: number;
}

export interface TribeData {
  preview: boolean;
  member_count: number;
  uuid: string;
  price_per_message: number;
  logo: string;
  name: string;
  unique_name: string;
  last_active: number;
}

const IS_DEV = window.location.host === "localhost:8080";

const formatUrl = (url: string): string => {
  if (url.includes("http://" || "https://")) {
    return url;
  }

  return IS_DEV ? `https://${url}` : `https://${url}`;
};

export async function get_tribes(
  url: string,
  uuid: string = "",
  search: string = "",
  page: number = 1,
  limit: number = 75
): Promise<TribeData[]> {
  let r;

  if (search) {
    r = await fetch(`${formatUrl(url)}/tribes?search=${search}`);
  } else if (uuid) {
    r = await fetch(`${formatUrl(url)}/tribes/${uuid}`);
  } else {
    r = await fetch(`${formatUrl(url)}/tribes?page=${page}&limit=${limit}`);
  }

  const result = await r.json();
  return result;
}

export async function get_people(url: string): Promise<Person[]> {
  const r = await fetch(`${formatUrl(url)}/people`);
  const result = await r.json();
  return result;
}

export async function get_tribes_total(url: string): Promise<number> {
  const r = await fetch(`${formatUrl(url)}/tribes/total`);
  const result = await r.json();
  return result;
}
