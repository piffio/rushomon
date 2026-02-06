import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = ({ url }) => {
	const token = url.searchParams.get('token');

	if (!token) {
		throw redirect(302, '/?error=missing_token');
	}

	// Store token in a secure cookie client-side
	// Note: This is set via JavaScript in +page.svelte
	// We return the token so the page component can handle it
	return {
		token
	};
};
