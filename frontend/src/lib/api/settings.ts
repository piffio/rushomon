import { apiClient } from "./client";

export interface ApiKey {
  id: string;
  name: string;
  hint: string;
  created_at: number;
  last_used_at: number | null;
  expires_at: number | null;
}

export interface ApiKeyCreateResponse extends ApiKey {
  raw_token: string;
}

export const apiKeysApi = {
  list: () => apiClient.get<ApiKey[]>("/api/settings/api-keys"),

  create: (name: string, expires_in_days: number | null) =>
    apiClient.post<ApiKeyCreateResponse>("/api/settings/api-keys", {
      name,
      expires_in_days
    }),

  revoke: (id: string) => apiClient.delete(`/api/settings/api-keys/${id}`)
};
