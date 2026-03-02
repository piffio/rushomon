import type { PageLoad } from './$types';
import type { User } from "$lib/types/api";

export const prerender = true; // Enable prerendering again

export const load: PageLoad = async () => {
	// For pre-rendered pricing page, we don't need user data
	// This page should be accessible to everyone including Google's validation bots
	return {
		user: undefined as User | undefined
	};
};
