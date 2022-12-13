import { writable } from "svelte/store";
import { localStorageStore } from "./storage";
import { Node, Stack, stack as initialStack } from "./nodes";
import { User, initialUsers } from "./users";
import type { Tribe, Person } from "./api/tribes";
import type { Channel } from "./api/lnd";

export const selectedNode = writable<Node>();

export const stack = writable<Stack>(initialStack);

export const users = writable<User[]>(initialUsers);

export const tribes = writable<Tribe[]>([]);

export const people = writable<Person[]>([]);

export const channels = writable<Channel[]>([]);
