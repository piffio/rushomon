import type { PageLoad } from './$types';
import { linksApi } from '$lib/api/links';
import { usageApi } from '$lib/api/usage';
import type { PaginatedResponse, Link, UsageResponse } from '$lib/types/api';

export const load: PageLoad = async ({ parent, url }) => {
	// Get user data from layout (now client-side)
	const parentData = await parent() as { user?: any; };
	const user = parentData.user;

	if (!user) {
		// This shouldn't happen if layout is working, but just in case
		return { user: null, paginatedLinks: null, usage: null };
	}

	try {
		// Get page from URL query params, default to 1
		const page = parseInt(url.searchParams.get('page') || '1', 10);

		// Fetch links and usage data in parallel
		const [paginatedLinks, usage] = await Promise.all([
			linksApi.list(page, 10),
			usageApi.getUsage().catch(() => null)
		]);

		return {
			user,
			paginatedLinks,
			usage
		};
	} catch (error) {
		// If links fetch fails, still return user data
		console.error('Failed to fetch links:', error);
		return {
			user,
			paginatedLinks: null,
			usage: null
		};
	}
};
