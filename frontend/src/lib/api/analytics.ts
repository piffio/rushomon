import { apiClient } from "./client";
import type { OrgAnalyticsResponse } from "$lib/types/api";

export const analyticsApi = {
  /**
   * Get org-level aggregate analytics for the last N days
   * @param days - Number of days to analyze (e.g., 7 for last 7 days, 0 for all time)
   */
  async getOrgAnalytics(days: number): Promise<OrgAnalyticsResponse> {
    return apiClient.get<OrgAnalyticsResponse>(
      `/api/analytics/org?days=${days}`
    );
  },

  /**
   * Get org-level aggregate analytics for a custom date range
   * @param start - Unix timestamp (seconds) for range start
   * @param end - Unix timestamp (seconds) for range end
   */
  async getOrgAnalyticsCustomRange(
    start: number,
    end: number
  ): Promise<OrgAnalyticsResponse> {
    return apiClient.get<OrgAnalyticsResponse>(
      `/api/analytics/org?start=${start}&end=${end}`
    );
  }
};
