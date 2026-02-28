import { apiClient } from './client';
import type {
	ListOrgsResponse,
	OrgDetails,
	OrgWithRole,
	OrgInvitation,
	InviteInfo,
	UsageResponse,
} from '$lib/types/api';

export const orgsApi = {
	async listMyOrgs(): Promise<ListOrgsResponse> {
		return apiClient.get<ListOrgsResponse>('/api/orgs');
	},

	async getOrg(id: string): Promise<OrgDetails> {
		return apiClient.get<OrgDetails>(`/api/orgs/${id}`);
	},

	async createOrg(name: string): Promise<{ org: OrgWithRole; role: string; }> {
		return apiClient.post<{ org: OrgWithRole; role: string; }>('/api/orgs', { name });
	},

	async updateOrgName(id: string, name: string): Promise<{ org: OrgWithRole; }> {
		return apiClient.patch<{ org: OrgWithRole; }>(`/api/orgs/${id}`, { name });
	},

	async switchOrg(org_id: string): Promise<{ org: OrgWithRole; }> {
		return apiClient.post<{ org: OrgWithRole; }>('/api/auth/switch-org', { org_id });
	},

	async inviteMember(org_id: string, email: string): Promise<{ invitation: OrgInvitation; }> {
		return apiClient.post<{ invitation: OrgInvitation; }>(`/api/orgs/${org_id}/invitations`, {
			email,
		});
	},

	async revokeInvitation(org_id: string, invitation_id: string): Promise<void> {
		return apiClient.delete<void>(`/api/orgs/${org_id}/invitations/${invitation_id}`);
	},

	async resendInvitation(org_id: string, invitation_id: string): Promise<void> {
		return apiClient.post<void>(`/api/orgs/${org_id}/invitations/${invitation_id}/resend`, {});
	},

	async removeMember(org_id: string, user_id: string): Promise<void> {
		return apiClient.delete<void>(`/api/orgs/${org_id}/members/${user_id}`);
	},

	async leaveOrg(org_id: string, user_id: string): Promise<void> {
		return apiClient.delete<void>(`/api/orgs/${org_id}/members/${user_id}`);
	},

	async getInviteInfo(token: string): Promise<InviteInfo> {
		return apiClient.get<InviteInfo>(`/api/invite/${token}`);
	},

	async acceptInvite(token: string): Promise<{ org: OrgWithRole; }> {
		return apiClient.post<{ org: OrgWithRole; }>(`/api/invite/${token}/accept`, {});
	},

	async deleteOrg(
		org_id: string,
		action: 'delete' | 'migrate',
		target_org_id?: string,
	): Promise<{ success: boolean; switched_to_org: OrgWithRole; }> {
		return apiClient.delete<{ success: boolean; switched_to_org: OrgWithRole; }>(
			`/api/orgs/${org_id}`,
			{ action, target_org_id },
		);
	},

	async getUsage(): Promise<UsageResponse> {
		return apiClient.get<UsageResponse>('/api/usage');
	},
};
