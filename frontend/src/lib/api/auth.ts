import type { User } from "$lib/types/api";
import { apiClient, clearAccessToken } from "./client";

export interface AuthProvider {
  name: string;
  label: string;
}

export const authApi = {
  /**
   * Get the current authenticated user
   * @returns User object if authenticated
   * @throws ApiError if not authenticated (401)
   */
  async me(): Promise<User> {
    return apiClient.get<User>("/api/auth/me");
  },

  /**
   * Logout the current user
   * Clears access token from localStorage and session from backend
   */
  async logout(): Promise<void> {
    // Clear access token from localStorage
    clearAccessToken();

    // Call backend to invalidate session and clear refresh cookie
    await apiClient.post<void>("/api/auth/logout");
  },

  /**
   * Fetch the list of enabled OAuth providers from the backend.
   * Only providers with a configured CLIENT_ID are returned.
   */
  async getProviders(): Promise<AuthProvider[]> {
    const result = await apiClient.get<{ providers: AuthProvider[] }>(
      "/api/auth/providers"
    );
    return result.providers;
  },

  /**
   * Get the login URL for a specific OAuth provider.
   * Browser should be redirected to this URL (not fetched via XHR).
   */
  getProviderLoginUrl(providerName: string, redirect?: string): string {
    const baseUrl = apiClient["baseUrl"];
    const url = new URL(`${baseUrl}/api/auth/${providerName}`);
    if (redirect) {
      url.searchParams.set("redirect", redirect);
    }
    return url.toString();
  },

  /**
   * @deprecated Use getProviderLoginUrl('github') instead.
   * Kept for backwards compatibility.
   */
  getLoginUrl(): string {
    return this.getProviderLoginUrl("github");
  },

  /**
   * Schedule account deletion (7-day grace period).
   * Requires confirmation string "DELETE" in the request body.
   * @returns Scheduled deletion timestamp and grace period info
   */
  async requestDeletion(): Promise<{
    success: boolean;
    message: string;
    scheduled_deletion_at: number;
    grace_period_seconds: number;
  }> {
    return apiClient.post<{
      success: boolean;
      message: string;
      scheduled_deletion_at: number;
      grace_period_seconds: number;
    }>("/api/auth/delete-account", { confirmation: "DELETE" });
  },

  /**
   * Cancel a pending account deletion.
   */
  async cancelDeletion(): Promise<{ success: boolean; message: string }> {
    return apiClient.post<{ success: boolean; message: string }>(
      "/api/auth/cancel-deletion"
    );
  },

  /**
   * Check if the current user has a pending account deletion.
   * @returns Deletion status with pending flag, scheduled date, and days remaining
   */
  async getDeletionStatus(): Promise<{
    pending: boolean;
    scheduled_deletion_at: number | null;
    days_remaining: number | null;
  }> {
    return apiClient.get<{
      pending: boolean;
      scheduled_deletion_at: number | null;
      days_remaining: number | null;
    }>("/api/auth/deletion-status");
  }
};
