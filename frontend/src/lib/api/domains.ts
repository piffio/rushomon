import { apiClient } from "./client";

export interface CustomDomain {
  id: string;
  org_id: string;
  hostname: string;
  status: "pending" | "active" | "failed";
  cf_hostname_id: string | null;
  created_at: number;
  verified_at: number | null;
}

export interface DnsInstructions {
  cname_target: string;
  txt_name: string | null;
  txt_value: string | null;
  needs_cname: boolean;
  needs_txt: boolean;
}

export interface CreateDomainResponse {
  domain: CustomDomain;
  dns_instructions: DnsInstructions;
}

export const domainsApi = {
  async listDomains(orgId: string): Promise<CustomDomain[]> {
    return apiClient.get<CustomDomain[]>(`/api/orgs/${orgId}/domains`);
  },

  async addDomain(
    orgId: string,
    hostname: string
  ): Promise<CreateDomainResponse> {
    return apiClient.post<CreateDomainResponse>(`/api/orgs/${orgId}/domains`, {
      hostname
    });
  },

  async deleteDomain(
    orgId: string,
    hostname: string
  ): Promise<{ deleted: boolean }> {
    return apiClient.delete<{ deleted: boolean }>(
      `/api/orgs/${orgId}/domains/${hostname}`
    );
  },

  async refreshDomain(orgId: string, hostname: string): Promise<CustomDomain> {
    return apiClient.post<CustomDomain>(
      `/api/orgs/${orgId}/domains/${hostname}/refresh`,
      {}
    );
  }
};
