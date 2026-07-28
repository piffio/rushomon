import type {
  InviteInfo,
  ListOrgsResponse,
  OrgDetails,
  OrgDomain,
  OrgInvitation,
  OrgSettings,
  OrgWithRole,
  UsageResponse
} from "$lib/types/api";
import { apiClient } from "./client";

export const orgsApi = {
  async listMyOrgs(): Promise<ListOrgsResponse> {
    return apiClient.get<ListOrgsResponse>("/api/orgs");
  },

  async getOrg(id: string): Promise<OrgDetails> {
    return apiClient.get<OrgDetails>(`/api/orgs/${id}`);
  },

  async createOrg(name: string): Promise<{ org: OrgWithRole; role: string }> {
    return apiClient.post<{ org: OrgWithRole; role: string }>("/api/orgs", {
      name
    });
  },

  async updateOrgName(id: string, name: string): Promise<{ org: OrgWithRole }> {
    return apiClient.patch<{ org: OrgWithRole }>(`/api/orgs/${id}`, { name });
  },

  async switchOrg(org_id: string): Promise<{ org: OrgWithRole }> {
    return apiClient.post<{ org: OrgWithRole }>("/api/auth/switch-org", {
      org_id
    });
  },

  async inviteMember(
    org_id: string,
    email: string,
    role: "member" | "admin" = "member"
  ): Promise<{ invitation: OrgInvitation }> {
    return apiClient.post<{ invitation: OrgInvitation }>(
      `/api/orgs/${org_id}/invitations`,
      { email, role }
    );
  },

  async revokeInvitation(org_id: string, invitation_id: string): Promise<void> {
    return apiClient.delete<void>(
      `/api/orgs/${org_id}/invitations/${invitation_id}`
    );
  },

  async resendInvitation(org_id: string, invitation_id: string): Promise<void> {
    return apiClient.post<void>(
      `/api/orgs/${org_id}/invitations/${invitation_id}/resend`,
      {}
    );
  },

  async removeMember(org_id: string, user_id: string): Promise<void> {
    return apiClient.delete<void>(`/api/orgs/${org_id}/members/${user_id}`);
  },

  async updateMemberRole(
    org_id: string,
    user_id: string,
    role: "member" | "admin"
  ): Promise<{ role: string }> {
    return apiClient.put<{ role: string }>(
      `/api/orgs/${org_id}/members/${user_id}/role`,
      { role }
    );
  },

  async leaveOrg(org_id: string, user_id: string): Promise<void> {
    return apiClient.delete<void>(`/api/orgs/${org_id}/members/${user_id}`);
  },

  async getInviteInfo(token: string): Promise<InviteInfo> {
    return apiClient.get<InviteInfo>(`/api/invite/${token}`);
  },

  async acceptInvite(token: string): Promise<{ org: OrgWithRole }> {
    return apiClient.post<{ org: OrgWithRole }>(
      `/api/invite/${token}/accept`,
      {}
    );
  },

  async deleteOrg(
    org_id: string,
    action: "delete" | "migrate",
    target_org_id?: string
  ): Promise<{ success: boolean; switched_to_org: OrgWithRole }> {
    return apiClient.delete<{ success: boolean; switched_to_org: OrgWithRole }>(
      `/api/orgs/${org_id}`,
      { action, target_org_id }
    );
  },

  async getUsage(): Promise<UsageResponse> {
    return apiClient.get<UsageResponse>("/api/usage");
  },

  async getOrgSettings(org_id: string): Promise<OrgSettings> {
    return apiClient.get<OrgSettings>(`/api/orgs/${org_id}/settings`);
  },

  async updateOrgSettings(
    org_id: string,
    settings: Partial<OrgSettings>
  ): Promise<OrgSettings> {
    return apiClient.patch<OrgSettings>(
      `/api/orgs/${org_id}/settings`,
      settings
    );
  },

  async uploadOrgLogo(
    org_id: string,
    file: File
  ): Promise<{ logo_url: string }> {
    const form = new FormData();
    form.append("logo", file);
    return apiClient.postForm<{ logo_url: string }>(
      `/api/orgs/${org_id}/logo`,
      form
    );
  },

  async deleteOrgLogo(org_id: string): Promise<void> {
    return apiClient.delete<void>(`/api/orgs/${org_id}/logo`);
  },

  async getOrgDomains(org_id: string): Promise<{ domains: OrgDomain[] }> {
    return apiClient.get<{ domains: OrgDomain[] }>(
      `/api/orgs/${org_id}/org-domains`
    );
  },

  async addOrgDomain(
    org_id: string,
    domain: string
  ): Promise<{
    domain: OrgDomain;
    instructions: string;
    token: string;
    verification_record: string;
  }> {
    return apiClient.post<{
      domain: OrgDomain;
      instructions: string;
      token: string;
      verification_record: string;
    }>(`/api/orgs/${org_id}/org-domains`, { domain });
  },

  async verifyOrgDomain(
    org_id: string,
    domain: string
  ): Promise<{ message: string }> {
    return apiClient.post<{ message: string }>(
      `/api/orgs/${org_id}/verify-org-domain`,
      { domain }
    );
  },

  async deleteOrgDomain(org_id: string, domain: string): Promise<void> {
    return apiClient.delete<void>(`/api/orgs/${org_id}/org-domains/${domain}`);
  }
};
