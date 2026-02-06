import { redirect } from '@sveltejs/kit';
import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch, url }) => {
	// Define public routes that don't require authentication
	const publicPaths = ['/auth/github', '/auth/callback'];
	const isPublicRoute = url.pathname === '/' || publicPaths.some((route: string) =>
		url.pathname.startsWith(route)
	);

	// For public routes, don't check auth
	if (isPublicRoute) {
		return {};
	}

	// For authenticated routes, validate the token and get user info
	try {
		const apiBaseUrl = PUBLIC_VITE_API_BASE_URL || 'http://localhost:8787';
		const response = await fetch(`${apiBaseUrl}/api/auth/me`, {
			credentials: 'include' // Send cookies automatically
		});

		if (!response.ok) {
			// Auth failed, redirect to home
			throw redirect(302, '/');
		}

		const user = await response.json();
		return { user };
	} catch (error) {
		// If it's already a redirect, re-throw it
		if (error instanceof Response || (error && typeof error === 'object' && 'status' in error && 'location' in error)) {
			throw error;
		}

		// For other errors (network, etc), redirect to home
		throw redirect(302, '/');
	}
};
