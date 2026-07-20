import { apiClient } from "./client";

export interface ApiKey {
  id: string;
  name: string;
  hint: string;
  created_at: number;
  last_used_at: number | null;
  expires_at: number | null;
  /** Org IDs this key is authorized to act on behalf of. Empty = legacy (all orgs). */
  org_ids: string[];
}

export interface ApiKeyCreateResponse extends ApiKey {
  raw_token: string;
}

export interface PublicSettings {
  founder_pricing_active: boolean;
  min_random_code_length: number;
  min_custom_code_length: number;
  system_min_code_length: number;
  active_discount_amount_pro_monthly: number;
  active_discount_amount_pro_annual: number;
  active_discount_amount_business_monthly: number;
  active_discount_amount_business_annual: number;
  email_notifications_enabled: boolean;
}

export const settingsApi = {
  getPublicSettings: () => apiClient.get<PublicSettings>("/api/settings")
};

export const apiKeysApi = {
  list: () => apiClient.get<ApiKey[]>("/api/settings/api-keys"),

  create: (name: string, expires_in_days: number | null, org_ids: string[]) =>
    apiClient.post<ApiKeyCreateResponse>("/api/settings/api-keys", {
      name,
      expires_in_days,
      org_ids
    }),

  revoke: (id: string) => apiClient.delete(`/api/settings/api-keys/${id}`),

  updateOrgs: (id: string, org_ids: string[]) =>
    apiClient.put<{ success: boolean }>(`/api/settings/api-keys/${id}/orgs`, {
      org_ids
    })
};
