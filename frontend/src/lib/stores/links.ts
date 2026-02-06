import { writable } from 'svelte/store';
import type { Link } from '$lib/types/api';

export const links = writable<Link[]>([]);
export const linksLoading = writable<boolean>(false);
export const currentPage = writable<number>(1);
export const linksPerPage = writable<number>(20);
