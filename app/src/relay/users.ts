export interface User {
  id: number;
  public_key: string;
  created_at: string;
  is_admin: boolean;
  deleted: number;
  alias?: string;
  route_hint?: string;
  person_uuid?: string;
  photo_url?: string;
  contact_key?: string;
}

export function formatPubkey(p: string): string {
  return `${p.substring(0, 4)}...${p.substring(p.length - 4)}}`;
}

export function formatRouteHint(r: string): string {
  return `${r.substring(0, 4)}...${r.substring(r.length - 4)}}`;
}

export const initialUsers = [];

// export const initialUsers: User[] = [
//   {
//     alias: "Evan",
//     pubkey:
//       "02290714deafd0cb33d2be3b634fc977a98a9c9fa1dd6c53cf17d99b350c08c67b",
//     balance: 250000,
//     routeHint:
//       "031863256ab7ce989e82453f38602018cd198753d8cb57d34224123449ba6c2f47:02736e7dad83d7205826649fc17db672ce08f8e87a2b47c7785ccbf79f24e91db0:1099729600212",
//   },
//   {
//     alias: "Kevin",
//     pubkey:
//       "03bfe6723c06fb2b7546df1e8ca1a17ae5c504615da32c945425ccbe8d3ca6260d",
//     balance: 133000,
//     routeHint: "",
//   },
//   {
//     alias: "Tomas",
//     pubkey:
//       "027dbce35947a3dafc826de411d97990e9b16e78512d1a9e70e87dcc788c2631db",
//     balance: 400000,
//     routeHint: "",
//   },
//   {
//     pubkey:
//       "02109ebcc86ef42f9261f820a6473d8a1e6c7dde10aa367b2f251c1b014b6ef256",
//     routeHint:
//       "02736e7dad83d7205826649fc17db672ce08f8e87a2b47c7785ccbf79f24e91db0:1099562287105",
//     balance: 10000,
//   },
//   {
//     pubkey:
//       "02c431e64078b10925584d64824c9d1d12eca05e2c56660ffa5ac84aa6946adfe5",
//     routeHint:
//       "02736e7dad83d7205826649fc17db672ce08f8e87a2b47c7785ccbf79f24e91db0:1099588239361",
//     balance: 100000,
//   },
// ];
