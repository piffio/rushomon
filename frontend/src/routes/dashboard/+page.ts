import type { PageLoad } from './$types';
import { apiClient } from '$lib/api/client';

export const load: PageLoad = async ({ parent }) => {
	// Get user data from layout (now client-side)
	const parentData = await parent() as { user?: any; };
	const user = parentData.user;

	if (!user) {
		// This shouldn't happen if layout is working, but just in case
		return { user: null, links: [] };
	}

	try {
		// Fetch links using apiClient (which adds Authorization header)
		const response = await apiClient.get<{ links: any[]; total: number; page: number; limit: number; }>('/api/links?page=1&limit=20');

		return {
			user,
			links: response.links || []
		};
	} catch (error) {
		// If links fetch fails, still return user data
		console.error('Failed to fetch links:', error);
		return {
			user,
			links: []
		};
	}
};
