import type { PageLoad } from "./$types";

export const prerender = true;

export const load: PageLoad = async () => {
	// 404 page doesn't need any data
	return {};
};
