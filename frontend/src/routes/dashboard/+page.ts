import type { PageLoad } from './$types';
import { linksApi } from '$lib/api/links';
import type { PaginatedResponse, Link } from '$lib/types/api';

export const load: PageLoad = async ({ parent, url }) => {
	// Get user data from layout (now client-side)
	const parentData = await parent() as { user?: any; };
	const user = parentData.user;

	if (!user) {
		// This shouldn't happen if layout is working, but just in case
		return { user: null, paginatedLinks: null };
	}

	try {
		// Get page from URL query params, default to 1
		const page = parseInt(url.searchParams.get('page') || '1', 10);

		// Fetch links with pagination using linksApi
		const paginatedLinks = await linksApi.list(page, 10);

		return {
			user,
			paginatedLinks
		};
	} catch (error) {
		// If links fetch fails, still return user data
		console.error('Failed to fetch links:', error);
		return {
			user,
			paginatedLinks: null
		};
	}
};
