import { apiClient } from './client';
import type { User } from '$lib/types/api';

export interface AdminUsersResponse {
	users: User[];
	total: number;
	page: number;
	limit: number;
}

export interface UpdateUserRoleRequest {
	role: 'admin' | 'member';
}

export interface SettingsResponse {
	[key: string]: string;
}

export interface UpdateSettingRequest {
	key: string;
	value: string;
}

export const adminApi = {
	/**
	 * List all users on the instance (admin only)
	 * @param page - Page number (default: 1)
	 * @param limit - Number of users per page (default: 50)
	 * @returns Paginated list of users
	 */
	async listUsers(page: number = 1, limit: number = 50): Promise<AdminUsersResponse> {
		return apiClient.get<AdminUsersResponse>(`/api/admin/users?page=${page}&limit=${limit}`);
	},

	/**
	 * Get a single user by ID (admin only)
	 * @param id - User UUID
	 * @returns User object
	 */
	async getUser(id: string): Promise<User> {
		return apiClient.get<User>(`/api/admin/users/${id}`);
	},

	/**
	 * Update a user's instance-level role (admin only)
	 * @param id - User UUID
	 * @param role - New role ('admin' or 'member')
	 * @returns Updated User object
	 */
	async updateUserRole(id: string, role: 'admin' | 'member'): Promise<User> {
		return apiClient.request<User>(`/api/admin/users/${id}`, {
			method: 'PUT',
			body: JSON.stringify({ role })
		});
	},

	/**
	 * Get all instance settings (admin only)
	 * @returns Settings key-value map
	 */
	async getSettings(): Promise<SettingsResponse> {
		return apiClient.get<SettingsResponse>('/api/admin/settings');
	},

	/**
	 * Update an instance setting (admin only)
	 * @param key - Setting key
	 * @param value - Setting value
	 * @returns Updated settings map
	 */
	async updateSetting(key: string, value: string): Promise<SettingsResponse> {
		return apiClient.request<SettingsResponse>('/api/admin/settings', {
			method: 'PUT',
			body: JSON.stringify({ key, value })
		});
	},

	/**
	 * Update an organization's tier (admin only)
	 * @param orgId - Organization UUID
	 * @param tier - New tier ('free' or 'unlimited')
	 * @returns Updated Organization object
	 */
	async updateOrgTier(orgId: string, tier: 'free' | 'unlimited'): Promise<{ id: string; tier: string; }> {
		return apiClient.request<{ id: string; tier: string; }>(`/api/admin/orgs/${orgId}/tier`, {
			method: 'PUT',
			body: JSON.stringify({ tier })
		});
	}
};
