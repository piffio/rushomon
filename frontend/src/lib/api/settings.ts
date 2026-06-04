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
