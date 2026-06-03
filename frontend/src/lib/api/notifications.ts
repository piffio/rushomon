import { apiClient } from "./client";
import type { NotificationPreferences } from "$lib/types/api";

export const notificationsApi = {
  /**
   * Get the current user's email notification preferences.
   * Missing preferences default to all-enabled on the backend.
   */
  async getPreferences(): Promise<NotificationPreferences> {
    return apiClient.get<NotificationPreferences>(
      "/api/notifications/preferences"
    );
  },

  /**
   * Update one or more notification preference flags.
   * Omitted fields retain their current values.
   */
  async updatePreferences(
    prefs: Partial<NotificationPreferences>
  ): Promise<NotificationPreferences> {
    return apiClient.patch<NotificationPreferences>(
      "/api/notifications/preferences",
      prefs
    );
  }
};
