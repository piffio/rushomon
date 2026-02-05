import { writable } from 'svelte/store';
import type { User } from '$lib/types/api';

export const currentUser = writable<User | null>(null);
export const isAuthenticated = writable<boolean>(false);
export const isLoading = writable<boolean>(true);
