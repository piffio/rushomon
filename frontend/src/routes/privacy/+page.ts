import type { PageLoad } from "./$types";

export const prerender = true;

export const load: PageLoad = async () => {
	// For pre-rendered privacy policy, we don't need user data
	// This page should be accessible to everyone including Google's validation bots
	return {
		user: null
	};
};
