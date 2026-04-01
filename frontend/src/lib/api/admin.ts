import type {
  AdminLinksResponse,
  AdminReportsResponse,
  BillingAccountDetails,
  BlacklistEntry,
  BlockDestinationResponse,
  LinkReport,
  ListBillingAccountsResponse,
  User
} from "$lib/types/api";
import { apiClient } from "./client";

export interface AdminUsersResponse {
  users: User[];
  total: number;
  page: number;
  limit: number;
  org_tiers: Record<string, string>;
}

export interface UpdateUserRoleRequest {
  role: "admin" | "member";
}

export interface SettingsResponse {
  [key: string]: string;
}

export interface UpdateSettingRequest {
  key: string;
  value: string;
}

export interface Discount {
  id: string;
  name: string;
  type: "fixed" | "percentage";
  amount?: number; // For fixed amount discounts
  basis_points?: number; // For percentage discounts (in hundredths of a percent)
  currency?: string;
  redemptions_count: number;
  max_redemptions: number | null;
  starts_at: string | null;
  ends_at: string | null;
  products?: Array<{
    id: string;
    name: string;
    recurring_interval: string;
    price_amount: number; // Price in cents
    price_currency: string;
  }>;
}

export interface DiscountsResponse {
  items: Discount[];
}

export interface Product {
  id: string;
  name: string;
  description: string | null;
  prices: Array<{
    id: string;
    price_amount: number; // Price in cents
    price_currency: string;
    recurring_interval: string | null;
    recurring_interval_count: number | null;
  }>;
  is_archived: boolean;
  created_at: string;
  modified_at: string | null;
}

export interface ProductsResponse {
  items: Product[];
}

export interface AdminApiKey {
  id: string;
  name: string;
  hint: string;
  user_id: string;
  user_email: string | null;
  user_name: string | null;
  org_id: string;
  org_name: string | null;
  tier: string | null;
  created_at: number;
  last_used_at: number | null;
  expires_at: number | null;
  status: "active" | "revoked" | "deleted";
  updated_at: number | null;
  updated_by: string | null;
}

export interface AdminApiKeysResponse {
  keys: AdminApiKey[];
  total: number;
  page: number;
  limit: number;
}

export const adminApi = {
  /**
   * List all users on the instance (admin only)
   * @param page - Page number (default: 1)
   * @param limit - Number of users per page (default: 50)
   * @returns Paginated list of users
   */
  async listUsers(
    page: number = 1,
    limit: number = 50
  ): Promise<AdminUsersResponse> {
    return apiClient.get<AdminUsersResponse>(
      `/api/admin/users?page=${page}&limit=${limit}`
    );
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
  async updateUserRole(id: string, role: "admin" | "member"): Promise<User> {
    return apiClient.request<User>(`/api/admin/users/${id}`, {
      method: "PUT",
      body: JSON.stringify({ role })
    });
  },

  /**
   * Get all instance settings (admin only)
   * @returns Settings key-value map
   */
  async getSettings(): Promise<SettingsResponse> {
    return apiClient.get<SettingsResponse>("/api/admin/settings");
  },

  /**
   * Update an instance setting (admin only)
   * @param key - Setting key
   * @param value - Setting value
   * @returns Updated settings map
   */
  async updateSetting(key: string, value: string): Promise<SettingsResponse> {
    return apiClient.request<SettingsResponse>("/api/admin/settings", {
      method: "PUT",
      body: JSON.stringify({ key, value })
    });
  },

  /**
   * Update an organization's tier (admin only)
   * @param orgId - Organization UUID
   * @param tier - New tier ('free', 'pro', 'business', or 'unlimited')
   * @returns Updated Organization object
   */
  /**
   * List all available Polar discounts (admin only)
   * @returns List of discounts from Polar
   */
  async listDiscounts(): Promise<DiscountsResponse> {
    return apiClient.get<DiscountsResponse>("/api/admin/discounts");
  },

  async listProducts(): Promise<ProductsResponse> {
    return apiClient.get<ProductsResponse>("/api/admin/products");
  },

  async syncProducts(): Promise<{
    success: boolean;
    message: string;
    products_count: number;
  }> {
    return apiClient.post<{
      success: boolean;
      message: string;
      products_count: number;
    }>("/api/admin/products/sync");
  },

  async saveProducts(): Promise<{
    success: boolean;
    message: string;
    products_count: number;
  }> {
    return apiClient.post<{
      success: boolean;
      message: string;
      products_count: number;
    }>("/api/admin/products/save");
  },

  async updateOrgTier(
    orgId: string,
    tier: string
  ): Promise<{ id: string; tier: string }> {
    return apiClient.request<{ id: string; tier: string }>(
      `/api/admin/orgs/${orgId}/tier`,
      {
        method: "PUT",
        body: JSON.stringify({ tier })
      }
    );
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
    if (org) params.set("org", org);
    if (email) params.set("email", email);
    if (domain) params.set("domain", domain);
    return apiClient.get<AdminLinksResponse>(`/api/admin/links?${params}`);
  },

  /**
   * Update a link's status (admin only)
   * @param id - Link UUID
   * @param status - New status ('active', 'disabled', or 'blocked')
   * @returns Success message
   */
  async updateLinkStatus(
    id: string,
    status: "active" | "disabled" | "blocked"
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/links/${id}`,
      {
        method: "PUT",
        body: JSON.stringify({ status })
      }
    );
  },

  /**
   * Delete a link (admin only)
   * @param id - Link UUID
   * @returns Success message
   */
  async deleteLink(id: string): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/links/${id}`,
      {
        method: "DELETE"
      }
    );
  },

  /**
   * Re-sync a link's KV entry (admin only)
   * @param id - Link UUID
   * @returns Success message
   */
  async syncLinkKv(id: string): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/links/${id}/sync-kv`,
      {
        method: "POST"
      }
    );
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
    matchType: "exact" | "domain" = "exact",
    reason: string
  ): Promise<BlockDestinationResponse> {
    return apiClient.request<BlockDestinationResponse>("/api/admin/blacklist", {
      method: "POST",
      body: JSON.stringify({ destination, match_type: matchType, reason })
    });
  },

  /**
   * Get all blacklist entries (admin only)
   * @returns List of blacklist entries
   */
  async getBlacklist(): Promise<BlacklistEntry[]> {
    return apiClient.get<BlacklistEntry[]>("/api/admin/blacklist");
  },

  /**
   * Remove a blacklist entry (admin only)
   * @param id - Blacklist entry UUID
   * @returns Success message
   */
  async removeBlacklistEntry(
    id: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/blacklist/${id}`,
      {
        method: "DELETE"
      }
    );
  },

  /**
   * Suspend a user (admin only)
   * @param id - User UUID
   * @param reason - Reason for suspension
   * @returns Success message with count of disabled links
   */
  async suspendUser(
    id: string,
    reason: string
  ): Promise<{ success: boolean; message: string; disabled_links: number }> {
    return apiClient.request<{
      success: boolean;
      message: string;
      disabled_links: number;
    }>(`/api/admin/users/${id}/suspend`, {
      method: "PUT",
      body: JSON.stringify({ reason })
    });
  },

  /**
   * Unsuspend a user (admin only)
   * @param id - User UUID
   * @returns Success message
   */
  async unsuspendUser(
    id: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/users/${id}/unsuspend`,
      {
        method: "PUT"
      }
    );
  },

  /**
   * Delete a user (admin only)
   * @param id - User UUID
   * @returns Success message with deletion counts
   */
  async deleteUser(id: string): Promise<{
    success: boolean;
    message: string;
    deleted_user_count: number;
    deleted_links_count: number;
    deleted_analytics_count: number;
  }> {
    return apiClient.request<{
      success: boolean;
      message: string;
      deleted_user_count: number;
      deleted_links_count: number;
      deleted_analytics_count: number;
    }>(`/api/admin/users/${id}`, {
      method: "DELETE",
      body: JSON.stringify({ confirmation: "DELETE" })
    });
  },

  /**
   * Submit an abuse report for a link (public endpoint, can be called by anyone)
   * @param linkId - Link UUID or short code
   * @param reason - Reason for the report
   * @param reporterEmail - Optional email of the reporter
   * @returns Success message
   */
  async reportLink(
    linkId: string,
    reason: string,
    reporterEmail?: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      "/api/reports/links",
      {
        method: "POST",
        body: JSON.stringify({
          link_id: linkId,
          reason,
          reporter_email: reporterEmail
        })
      }
    );
  },

  /**
   * Get all abuse reports (admin only)
   * @param page - Page number (default: 1)
   * @param limit - Number of reports per page (default: 50)
   * @param status - Filter by status ('pending', 'reviewed', 'dismissed')
   * @returns Paginated list of reports
   */
  async getReports(
    page: number = 1,
    limit: number = 50,
    status?: string
  ): Promise<AdminReportsResponse> {
    const params = new URLSearchParams({
      page: page.toString(),
      limit: limit.toString()
    });
    if (status) params.append("status", status);

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
  async updateReportStatus(
    id: string,
    status: "reviewed" | "dismissed",
    adminNotes?: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/reports/${id}`,
      {
        method: "PUT",
        body: JSON.stringify({ status, admin_notes: adminNotes })
      }
    );
  },

  /**
   * Get count of pending reports (admin only)
   * @returns Number of pending reports
   */
  async getPendingReportsCount(): Promise<{ count: number }> {
    return apiClient.get<{ count: number }>("/api/admin/reports/pending/count");
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
    if (search) params.set("search", search);
    if (tier) params.set("tier", tier);
    return apiClient.get<ListBillingAccountsResponse>(
      `/api/admin/billing-accounts?${params}`
    );
  },

  /**
   * Get billing account details (admin only)
   * @param id - Billing account ID
   * @returns Detailed billing account view with orgs and usage
   */
  async getBillingAccount(id: string): Promise<BillingAccountDetails> {
    return apiClient.get<BillingAccountDetails>(
      `/api/admin/billing-accounts/${id}`
    );
  },

  /**
   * Update billing account tier (admin only)
   * @param id - Billing account ID
   * @param tier - New tier ('free', 'pro', 'business', or 'unlimited')
   * @returns Success response
   */
  async updateBillingAccountTier(
    id: string,
    tier: string
  ): Promise<{ success: boolean; message: string; tier: string }> {
    return apiClient.request<{
      success: boolean;
      message: string;
      tier: string;
    }>(`/api/admin/billing-accounts/${id}/tier`, {
      method: "PUT",
      body: JSON.stringify({ tier })
    });
  },

  /**
   * Reset billing account counter for current month (admin only)
   * @param id - Billing account ID
   * @returns Success response
   */
  async resetBillingAccountCounter(
    id: string
  ): Promise<{ success: boolean; message: string; year_month: string }> {
    return apiClient.request<{
      success: boolean;
      message: string;
      year_month: string;
    }>(`/api/admin/billing-accounts/${id}/reset-counter`, {
      method: "POST"
    });
  },

  /**
   * Update subscription status for a billing account (admin only)
   * @param id - Billing account ID
   * @param status - New subscription status ('active', 'canceled', etc.)
   * @returns Success response
   */
  async updateSubscriptionStatus(
    id: string,
    status: string
  ): Promise<{
    success: boolean;
    message: string;
    subscription_id: string;
    new_status: string;
  }> {
    return apiClient.request<{
      success: boolean;
      message: string;
      subscription_id: string;
      new_status: string;
    }>(`/api/admin/billing-accounts/${id}/subscription`, {
      method: "PUT",
      body: JSON.stringify({ status })
    });
  },

  /**
   * Trigger the expired subscription downgrade cron job (admin only)
   * @returns Object with processed, success, and error counts
   */
  async triggerCronDowngrade(): Promise<{
    processed: number;
    success: number;
    errors: number;
  }> {
    return apiClient.request<{
      processed: number;
      success: number;
      errors: number;
    }>("/api/admin/cron/trigger-downgrade", {
      method: "POST"
    });
  },

  /**
   * List all API keys instance-wide (admin only)
   */
  async listApiKeys(
    page: number = 1,
    limit: number = 20,
    search?: string,
    status?: "all" | "active" | "revoked" | "deleted"
  ): Promise<AdminApiKeysResponse> {
    const params = new URLSearchParams({
      page: page.toString(),
      limit: limit.toString()
    });
    if (search) params.set("search", search);
    if (status && status !== "all") params.set("status", status);
    return apiClient.get<AdminApiKeysResponse>(`/api/admin/api-keys?${params}`);
  },

  /**
   * Revoke an API key by ID (admin only, soft delete)
   */
  async revokeApiKey(
    id: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/api-keys/${id}`,
      {
        method: "DELETE"
      }
    );
  },

  /**
   * Reactivate a revoked API key by ID (admin only)
   */
  async reactivateApiKey(
    id: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/api-keys/${id}/reactivate`,
      { method: "POST" }
    );
  },

  /**
   * Soft delete an API key by ID (admin only)
   */
  async deleteApiKey(
    id: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/api-keys/${id}/delete`,
      { method: "POST" }
    );
  },

  /**
   * Restore a deleted API key by ID (admin only)
   */
  async restoreApiKey(
    id: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      `/api/admin/api-keys/${id}/restore`,
      { method: "POST" }
    );
  }
};
