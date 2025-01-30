export interface Tribe {
  app_url: string;
  badges: [];
  bots: string;
  created: Date;
  deleted: boolean;
  description: string;
  escrow_amount: number;
  escrow_millis: number;
  feed_type: number;
  feed_url: string;
  group_key: string;
  img: string;
  last_active: number;
  member_count: number;
  name: string;
  owner_alias: string;
  owner_pubkey: string;
  owner_route_hint: string;
  pin: string;
  preview: string;
  price_per_message: number;
  price_to_join: number;
  private: boolean;
  profile_filters: string;
  tags: string[];
  unique_name: string;
  unlisted: boolean;
  updated: Date;
  uuid: string;
}

export interface Remote {
  host: string;
  note?: string;
  ec2?: string;
  default_host?: string;
  ec2_instance_id: string;
  public_ip_address?: string;
}

export interface ILightningBot {
  balance_in_msat: number;
  label: string;
  id: string;
  contact_info: string;
  network: string;
  error_message: string;
  alias: string;
}
