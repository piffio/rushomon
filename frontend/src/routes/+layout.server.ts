import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import type { LayoutServerData } from './$types';

export const load = async ({ cookies, url }: { cookies: any, url: any; }) => {
	const sessionToken = cookies.get('rushomon_session');

	// Define public routes that don't require authentication
	const publicPaths = ['/auth/github', '/auth/callback'];
	const isPublicRoute = url.pathname === '/' || publicPaths.some((route: string) =>
		url.pathname.startsWith(route)
	);

	// If accessing protected route without session, redirect to home
	if (!isPublicRoute && !sessionToken) {
		throw redirect(302, '/');
	}

	// For authenticated routes (non-public), validate the token and get user info
	if (!isPublicRoute && sessionToken) {
		const apiBaseUrl = env.PUBLIC_VITE_API_BASE_URL || 'http://localhost:8787';
		const response = await fetch(`${apiBaseUrl}/api/auth/me`, {
			headers: {
				'Cookie': `rushomon_session=${sessionToken}`
			}
		});

		if (!response.ok) {
			throw redirect(302, '/');
		}

		const user = await response.json();
		return { user };
	}

	// For public routes, don't return user data
	return {};
};
