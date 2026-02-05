import type { PageLoad } from './$types';

export const load: PageLoad = async ({ parent, fetch }) => {
	// Get user data from layout server
	const parentData = await parent() as { user?: any; };
	const user = parentData.user;

	if (!user) {
		// This shouldn't happen if layout server is working, but just in case
		return { user: null, links: [] };
	}

	try {
		// Fetch links using the same pattern as layout server
		// We need to get the session cookie and pass it explicitly
		const response = await fetch('http://localhost:8787/api/links?page=1&limit=20', {
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include'
		});

		if (!response.ok) {
			throw new Error(`HTTP ${response.status}`);
		}

		const links = await response.json();

		return {
			user,
			links
		};
	} catch (error) {
		// If links fetch fails, still return user data
		return {
			user,
			links: []
		};
	}
};
