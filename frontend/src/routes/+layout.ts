import { redirect } from "@sveltejs/kit";
import type { LayoutLoad } from "./$types";
import { browser } from "$app/environment";
import { setSvelteKitFetch } from "$lib/api/client";

export const load: LayoutLoad = async ({ url, fetch }) => {
  // Set up SvelteKit fetch for API client to use
  setSvelteKitFetch(fetch);

  // Define public routes that don't require authentication
  const publicPaths = [
    "/auth/github",
    "/auth/callback",
    "/pricing",
    "/terms",
    "/privacy",
    "/report",
    "/login",
    "/invite"
  ];
  const isPublicRoute =
    url.pathname === "/" ||
    publicPaths.some((route: string) => url.pathname.startsWith(route));

  // For SSR, we can't access localStorage, so skip auth checks
  // Auth will be handled client-side
  if (!browser) {
    // For SSR, return empty - auth will be handled client-side
    return {};
  }

  try {
    // Import apiClient dynamically to ensure it runs client-side
    const { authApi } = await import("$lib/api/auth");
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
      throw redirect(302, "/");
    }

    // For other errors on protected routes, also redirect to home
    throw redirect(302, "/");
  }
};
