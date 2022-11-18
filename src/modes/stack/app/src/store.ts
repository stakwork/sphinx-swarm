import { writable } from "svelte/store";
import { localStorageStore } from "./storage";
import { Node, Stack, stack as initialStack } from "./nodes";
import { User, initialUsers } from "./users";
import { Tribes, initialTribes } from "./tribes";

export const selectedNode = writable<Node>();

export const stack = writable<Stack>(initialStack);

export const users = writable<User[]>(initialUsers);

export const tribes = writable<Tribes[]>(initialTribes);

export const nodeStore = writable<String>("");

export const nodeConnections = writable<String>("");
