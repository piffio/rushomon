import { redirect } from '@sveltejs/kit';
import type { LayoutServerData } from './$types';

export const load = async ({ cookies, url }: { cookies: any, url: any; }) => {
	// For protected routes, check if user is authenticated
	const sessionToken = cookies.get('rushomon_session');

	// If accessing protected routes without session, redirect to home
	if (url.pathname.startsWith('/dashboard') && !sessionToken) {
		throw redirect(302, '/');
	}

	// For dashboard, validate the token and get user info
	if (url.pathname.startsWith('/dashboard') && sessionToken) {
		try {
			const response = await fetch('http://localhost:8787/api/auth/me', {
				headers: {
					'Cookie': `rushomon_session=${sessionToken}`
				}
			});

			if (!response.ok) {
				throw redirect(302, '/');
			}

			const user = await response.json();
			return { user };
		} catch (error) {
			throw redirect(302, '/');
		}
	}

	return {};
};
