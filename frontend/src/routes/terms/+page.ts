import type { PageLoad } from './$types';

export const prerender = true;

export const load: PageLoad = async () => {
	// For pre-rendered terms page, we don't need user data
	// This page should be accessible to everyone including validation bots
	return {
		user: null
	};
};
