import type { PageLoad } from './$types';
import { linksApi } from '$lib/api/links';
import type { Link, LinkAnalyticsResponse } from '$lib/types/api';

export const load: PageLoad = async ({ params, parent, url }) => {
	const parentData = await parent() as { user?: any; };
	const user = parentData.user;

	if (!user) {
		return { user: null, link: null, analytics: null, error: null };
	}

	try {
		// Look up link by short_code
		const link: Link = await linksApi.getByCode(params.short_code);

		// Parse time range from URL query params (or use defaults)
		const now = Math.floor(Date.now() / 1000);
		const days = parseInt(url.searchParams.get('days') || '7', 10);
		const start = days === 0 ? 0 : now - days * 24 * 60 * 60; // 0 = All time
		const end = now;

		// Fetch analytics
		const analytics: LinkAnalyticsResponse = await linksApi.getAnalytics(link.id, start, end);

		return { user, link, analytics, days, error: null };
	} catch (error: any) {
		console.error('Failed to load link analytics:', error);
		return {
			user,
			link: null,
			analytics: null,
			days: 7,
			error: error?.message || 'Failed to load analytics'
		};
	}
};
