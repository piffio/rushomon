import { apiClient, clearAccessToken } from './client';
import type { User } from '$lib/types/api';

export const authApi = {
	/**
	 * Get the current authenticated user
	 * @returns User object if authenticated
	 * @throws ApiError if not authenticated (401)
	 */
	async me(): Promise<User> {
		return apiClient.get<User>('/api/auth/me');
	},

	/**
	 * Logout the current user
	 * Clears access token from localStorage and session from backend
	 */
	async logout(): Promise<void> {
		// Clear access token from localStorage
		clearAccessToken();

		// Call backend to invalidate session and clear refresh cookie
		await apiClient.post<void>('/api/auth/logout');
	},

	/**
	 * Initiate GitHub OAuth login
	 * This should be called by redirecting the browser, not via fetch
	 * @returns The URL to redirect to
	 */
	getLoginUrl(): string {
		const baseUrl = apiClient['baseUrl'];
		return `${baseUrl}/api/auth/github`;
	}
};
