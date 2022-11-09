import { writable } from "svelte/store";
import { localStorageStore } from "./storage";
import { Node, Stack, stack as initialStack } from "./nodes";

export const selectedNode = writable<Node>();

export const stack = writable<Stack>(initialStack);
