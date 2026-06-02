import { apiClient } from "./client";

export interface CustomDomain {
  id: string;
  org_id: string;
  hostname: string;
  status: "pending" | "active" | "failed" | "inactive_downgrade";
  cf_hostname_id: string | null;
  ssl_status: "pending" | "active" | "failed";
  created_at: number;
  verified_at: number | null;
}

export interface DnsInstructions {
  cname_target: string;
  txt_records: TxtRecord[];
  needs_cname: boolean;
  needs_txt: boolean;
}

export interface TxtRecord {
  name: string;
  value: string;
  purpose: "ownership" | "ssl_validation";
}

export interface DomainWithInstructions {
  domain: CustomDomain;
  dns_instructions: DnsInstructions | null;
}

export type CreateDomainResponse = DomainWithInstructions;

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
  ): Promise<{
    deleted: boolean;
    cf_deleted: boolean;
    cf_deleted_message?: string;
  }> {
    return apiClient.delete<{
      deleted: boolean;
      cf_deleted: boolean;
      cf_deleted_message?: string;
    }>(`/api/orgs/${orgId}/domains/${hostname}`);
  },

  async refreshDomain(
    orgId: string,
    hostname: string
  ): Promise<DomainWithInstructions> {
    return apiClient.post<DomainWithInstructions>(
      `/api/orgs/${orgId}/domains/${hostname}/refresh`,
      {}
    );
  }
};
