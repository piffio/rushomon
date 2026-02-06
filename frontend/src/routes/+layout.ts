import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { browser } from '$app/environment';

// Mark this as client-side only to avoid SSR issues with localStorage
export const ssr = false;

export const load: LayoutLoad = async ({ url }) => {
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
	// This now runs client-side only, so localStorage is available
	if (browser) {
		try {
			// Import apiClient dynamically to ensure it runs client-side
			const { authApi } = await import('$lib/api/auth');
			const user = await authApi.me();
			return { user };
		} catch (error: any) {
			// Auth failed, redirect to home
			if (error?.status === 401) {
				throw redirect(302, '/');
			}

			// For other errors, also redirect to home
			throw redirect(302, '/');
		}
	}

	// If somehow we're not in browser (shouldn't happen with ssr=false), return empty
	return {};
};
