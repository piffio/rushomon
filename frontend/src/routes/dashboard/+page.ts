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
		return {
			user: null,
			paginatedLinks: null,
			usage: null,
			initialSearch: '',
			initialStatus: 'all',
			initialSort: 'created'
		};
	}

	try {
		// Get params from URL query params
		const page = parseInt(url.searchParams.get('page') || '1', 10);
		const search = url.searchParams.get('search') || '';
		const status = url.searchParams.get('status') as 'active' | 'disabled' | null;
		const sort = (url.searchParams.get('sort') || 'created') as 'created' | 'updated' | 'clicks' | 'title' | 'code';

		// Fetch links and usage data in parallel
		const [paginatedLinks, usage] = await Promise.all([
			linksApi.list(page, 10, search || undefined, status || undefined, sort),
			usageApi.getUsage().catch(() => null)
		]);

		return {
			user,
			paginatedLinks,
			usage,
			initialSearch: search,
			initialStatus: status || 'all',
			initialSort: sort
		};
	} catch (error) {
		// If links fetch fails, still return user data
		console.error('Failed to fetch links:', error);
		return {
			user,
			paginatedLinks: null,
			usage: null,
			initialSearch: '',
			initialStatus: 'all',
			initialSort: 'created'
		};
	}
};
