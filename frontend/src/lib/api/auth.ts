import { apiClient } from './client';
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
	 * Clears session from backend and expires cookie
	 */
	async logout(): Promise<void> {
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
