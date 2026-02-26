import { apiClient } from './client';
import type {
	User,
	Link,
	AdminLink,
	BlacklistEntry,
	BlockDestinationRequest,
	BlockDestinationResponse,
	AdminLinksResponse,
	LinkReport,
	LinkReportWithLink,
	AdminReportsResponse,
	UpdateReportStatusRequest,
	BillingAccountWithStats,
	BillingAccountDetails,
	ListBillingAccountsResponse,
} from "$lib/types/api";

export interface AdminUsersResponse {
	users: User[];
	total: number;
	page: number;
	limit: number;
	org_tiers: Record<string, string>;
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
	},

	/**
	 * List all links for admin moderation (admin only)
	 * @param page - Page number (default: 1)
	 * @param limit - Number of links per page (default: 50)
	 * @param org - Optional org filter
	 * @param email - Optional email filter
	 * @param domain - Optional destination domain filter
	 * @returns Paginated list of links with creator and org info
	 */
	async listLinks(
		page: number = 1,
		limit: number = 50,
		org?: string,
		email?: string,
		domain?: string
	): Promise<AdminLinksResponse> {
		const params = new URLSearchParams({
			page: page.toString(),
			limit: limit.toString()
		});
		if (org) params.set('org', org);
		if (email) params.set('email', email);
		if (domain) params.set('domain', domain);
		return apiClient.get<AdminLinksResponse>(`/api/admin/links?${params}`);
	},

	/**
	 * Update a link's status (admin only)
	 * @param id - Link UUID
	 * @param status - New status ('active', 'disabled', or 'blocked')
	 * @returns Success message
	 */
	async updateLinkStatus(id: string, status: 'active' | 'disabled' | 'blocked'): Promise<{ success: boolean; message: string; }> {
		return apiClient.request<{ success: boolean; message: string; }>(`/api/admin/links/${id}`, {
			method: 'PUT',
			body: JSON.stringify({ status })
		});
	},

	/**
	 * Delete a link (admin only)
	 * @param id - Link UUID
	 * @returns Success message
	 */
	async deleteLink(id: string): Promise<{ success: boolean; message: string; }> {
		return apiClient.request<{ success: boolean; message: string; }>(`/api/admin/links/${id}`, {
			method: 'DELETE'
		});
	},

	/**
	 * Block a destination URL (admin only)
	 * @param destination - Destination URL or domain
	 * @param matchType - Match type ('exact' or 'domain')
	 * @param reason - Reason for blocking
	 * @returns Success message with count of blocked links
	 */
	async blockDestination(
		destination: string,
		matchType: 'exact' | 'domain' = 'exact',
		reason: string
	): Promise<BlockDestinationResponse> {
		return apiClient.request<BlockDestinationResponse>('/api/admin/blacklist', {
			method: 'POST',
			body: JSON.stringify({ destination, match_type: matchType, reason })
		});
	},

	/**
	 * Get all blacklist entries (admin only)
	 * @returns List of blacklist entries
	 */
	async getBlacklist(): Promise<BlacklistEntry[]> {
		return apiClient.get<BlacklistEntry[]>('/api/admin/blacklist');
	},

	/**
	 * Remove a blacklist entry (admin only)
	 * @param id - Blacklist entry UUID
	 * @returns Success message
	 */
	async removeBlacklistEntry(id: string): Promise<{ success: boolean; message: string; }> {
		return apiClient.request<{ success: boolean; message: string; }>(`/api/admin/blacklist/${id}`, {
			method: 'DELETE'
		});
	},

	/**
	 * Suspend a user (admin only)
	 * @param id - User UUID
	 * @param reason - Reason for suspension
	 * @returns Success message with count of disabled links
	 */
	async suspendUser(id: string, reason: string): Promise<{ success: boolean; message: string; disabled_links: number; }> {
		return apiClient.request<{ success: boolean; message: string; disabled_links: number; }>(`/api/admin/users/${id}/suspend`, {
			method: 'PUT',
			body: JSON.stringify({ reason })
		});
	},

	/**
	 * Unsuspend a user (admin only)
	 * @param id - User UUID
	 * @returns Success message
	 */
	async unsuspendUser(id: string): Promise<{ success: boolean; message: string; }> {
		return apiClient.request<{ success: boolean; message: string; }>(`/api/admin/users/${id}/unsuspend`, {
			method: 'PUT'
		});
	},

	/**
	 * Submit an abuse report for a link (public endpoint, can be called by anyone)
	 * @param linkId - Link UUID or short code
	 * @param reason - Reason for the report
	 * @param reporterEmail - Optional email of the reporter
	 * @returns Success message
	 */
	async reportLink(linkId: string, reason: string, reporterEmail?: string): Promise<{ success: boolean; message: string; }> {
		return apiClient.request<{ success: boolean; message: string; }>('/api/reports/links', {
			method: 'POST',
			body: JSON.stringify({ link_id: linkId, reason, reporter_email: reporterEmail })
		});
	},

	/**
	 * Get all abuse reports (admin only)
	 * @param page - Page number (default: 1)
	 * @param limit - Number of reports per page (default: 50)
	 * @param status - Filter by status ('pending', 'reviewed', 'dismissed')
	 * @returns Paginated list of reports
	 */
	async getReports(page: number = 1, limit: number = 50, status?: string): Promise<AdminReportsResponse> {
		const params = new URLSearchParams({
			page: page.toString(),
			limit: limit.toString(),
		});
		if (status) params.append('status', status);

		return apiClient.get<AdminReportsResponse>(`/api/admin/reports?${params}`);
	},

	/**
	 * Get a single abuse report by ID (admin only)
	 * @param id - Report UUID
	 * @returns Report details
	 */
	async getReport(id: string): Promise<LinkReport> {
		return apiClient.get<LinkReport>(`/api/admin/reports/${id}`);
	},

	/**
	 * Update report status (admin only)
	 * @param id - Report UUID
	 * @param status - New status ('reviewed' or 'dismissed')
	 * @param adminNotes - Optional admin notes
	 * @returns Success message
	 */
	async updateReportStatus(id: string, status: 'reviewed' | 'dismissed', adminNotes?: string): Promise<{ success: boolean; message: string; }> {
		return apiClient.request<{ success: boolean; message: string; }>(`/api/admin/reports/${id}`, {
			method: 'PUT',
			body: JSON.stringify({ status, admin_notes: adminNotes })
		});
	},

	/**
	 * Get count of pending reports (admin only)
	 * @returns Number of pending reports
	 */
	async getPendingReportsCount(): Promise<{ count: number; }> {
		return apiClient.get<{ count: number; }>('/api/admin/reports/pending/count');
	},

	/**
	 * List all billing accounts (admin only)
	 * @param page - Page number (default: 1)
	 * @param limit - Number of accounts per page (default: 50)
	 * @param search - Optional email search filter
	 * @param tier - Optional tier filter
	 * @returns Paginated list of billing accounts with stats
	 */
	async listBillingAccounts(
		page: number = 1,
		limit: number = 50,
		search?: string,
		tier?: string
	): Promise<ListBillingAccountsResponse> {
		const params = new URLSearchParams({
			page: page.toString(),
			limit: limit.toString()
		});
		if (search) params.set('search', search);
		if (tier) params.set('tier', tier);
		return apiClient.get<ListBillingAccountsResponse>(`/api/admin/billing-accounts?${params}`);
	},

	/**
	 * Get billing account details (admin only)
	 * @param id - Billing account ID
	 * @returns Detailed billing account view with orgs and usage
	 */
	async getBillingAccount(id: string): Promise<BillingAccountDetails> {
		return apiClient.get<BillingAccountDetails>(`/api/admin/billing-accounts/${id}`);
	},

	/**
	 * Update billing account tier (admin only)
	 * @param id - Billing account ID
	 * @param tier - New tier ('free', 'pro', 'business', or 'unlimited')
	 * @returns Success response
	 */
	async updateBillingAccountTier(id: string, tier: string): Promise<{ success: boolean; message: string; tier: string; }> {
		return apiClient.request<{ success: boolean; message: string; tier: string; }>(`/api/admin/billing-accounts/${id}/tier`, {
			method: 'PUT',
			body: JSON.stringify({ tier })
		});
	},

	/**
	 * Reset billing account counter for current month (admin only)
	 * @param id - Billing account ID
	 * @returns Success response
	 */
	async resetBillingAccountCounter(id: string): Promise<{ success: boolean; message: string; year_month: string; }> {
		return apiClient.request<{ success: boolean; message: string; year_month: string; }>(`/api/admin/billing-accounts/${id}/reset-counter`, {
			method: 'POST'
		});
	}
};
