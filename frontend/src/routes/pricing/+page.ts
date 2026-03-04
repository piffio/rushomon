import type { PageLoad } from './$types';
import type { User } from "$lib/types/api";

// export const prerender = false; // Disable prerendering to allow dynamic settings

export const load: PageLoad = async () => {
	// Static adapter - no server-side data fetching
	return {
		user: undefined as User | undefined,
		settings: null
	};
};
