import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { browser } from '$app/environment';

// Mark this as client-side only to avoid SSR issues with localStorage
export const ssr = false;

export const load: LayoutLoad = async ({ url }) => {
	// Define public routes that don't require authentication
	const publicPaths = ['/auth/github', '/auth/callback', '/pricing', '/terms', '/report', '/login'];
	const isPublicRoute = url.pathname === '/' || publicPaths.some((route: string) =>
		url.pathname.startsWith(route)
	);

	// This now runs client-side only, so localStorage is available
	if (browser) {
		try {
			// Import apiClient dynamically to ensure it runs client-side
			const { authApi } = await import('$lib/api/auth');
			const user = await authApi.me();

			// For public routes, return user if authenticated but don't redirect
			// For protected routes, return user (already authenticated)
			return { user };
		} catch (error: any) {
			// For public routes, auth failure is okay - just don't include user
			if (isPublicRoute) {
				return {};
			}

			// For protected routes, auth failed - redirect to home
			if (error?.status === 401) {
				throw redirect(302, '/');
			}

			// For other errors on protected routes, also redirect to home
			throw redirect(302, '/');
		}
	}

	// If somehow we're not in browser (shouldn't happen with ssr=false), return empty
	return {};
};
