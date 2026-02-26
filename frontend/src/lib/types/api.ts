export interface User {
	id: string;
	email: string;
	name: string | null;
	avatar_url: string | null;
	oauth_provider: string;
	oauth_id: string;
	org_id: string;
	role: 'admin' | 'member';
	created_at: number;
	suspended_at: number | null;
	suspension_reason: string | null;
	suspended_by: string | null;
	billing_account_id?: string | null;
	billing_account_tier?: string | null;
}

export type LinkStatus = 'active' | 'disabled' | 'blocked';

export interface Link {
	id: string;
	org_id: string;
	short_code: string;
	destination_url: string;
	title: string | null;
	created_by: string;
	created_at: number;
	updated_at: number | null;
	expires_at: number | null;
	status: LinkStatus;
	click_count: number;
	tags: string[];
}

export interface TagWithCount {
	name: string;
	count: number;
}

export interface CreateLinkRequest {
	destination_url: string;
	short_code?: string;
	title?: string;
	expires_at?: number;
	tags?: string[];
}

export interface UpdateLinkRequest {
	destination_url?: string;
	title?: string;
	expires_at?: number;
	status?: LinkStatus;
	tags?: string[];
}

export interface ApiError {
	message: string;
	status: number;
	data?: any; // Full error response data for detailed error handling
}

export interface PaginationMeta {
	page: number;
	limit: number;
	total: number;
	total_pages: number;
	has_next: boolean;
	has_prev: boolean;
}

export interface DashboardStats {
	total_links: number;
	active_links: number;
	total_clicks: number;
}

export interface PaginatedResponse<T> {
	data: T[];
	pagination: PaginationMeta;
	stats?: DashboardStats;
}

export interface ClicksOverTime {
	date: string;
	count: number;
}

export interface ReferrerCount {
	referrer: string;
	count: number;
}

export interface CountryCount {
	country: string;
	count: number;
}

export interface UserAgentCount {
	user_agent: string;
	count: number;
}

export interface LinkAnalyticsResponse {
	link: Link;
	total_clicks_in_range: number;
	clicks_over_time: ClicksOverTime[];
	top_referrers: ReferrerCount[];
	top_countries: CountryCount[];
	top_user_agents: UserAgentCount[];
	analytics_gated?: boolean;
	gated_reason?: string;
}

export interface UsageResponse {
	tier: string;
	limits: {
		max_links_per_month: number | null;
		analytics_retention_days: number | null;
		allow_custom_short_code: boolean;
	};
	usage: {
		links_created_this_month: number;
	};
}

// Admin moderation types
export interface AdminLink {
	id: string;
	org_id: string;
	short_code: string;
	destination_url: string;
	title: string | null;
	created_by: string;
	created_at: number;
	updated_at: number | null;
	expires_at: number | null;
	status: LinkStatus;
	click_count: number;
	creator_email: string;
	org_name: string;
}

export interface BlacklistEntry {
	id: string;
	destination: string;
	match_type: 'exact' | 'domain';
	reason: string;
	created_by: string;
	created_at: number;
}

export interface BlockDestinationRequest {
	destination: string;
	match_type?: 'exact' | 'domain';
	reason: string;
}

export interface BlockDestinationResponse {
	success: boolean;
	message: string;
	blocked_links: number;
	already_blocked?: boolean;
}

export interface SuspendUserRequest {
	reason: string;
}

export interface UpdateLinkStatusRequest {
	status: LinkStatus;
}

export interface LinkReport {
	id: string;
	link_id: string;
	reason: string;
	reporter_user_id?: string;
	reporter_email?: string;
	status: 'pending' | 'reviewed' | 'dismissed';
	admin_notes?: string;
	reviewed_by?: string;
	reviewed_at?: number;
	created_at: number;
}

export interface LinkReportWithLink {
	id: string;
	link_id: string;
	link: AdminLink;
	reason: string;
	reporter_user_id?: string;
	reporter_email?: string;
	status: 'pending' | 'reviewed' | 'dismissed';
	admin_notes?: string;
	reviewed_by?: string;
	reviewed_at?: number;
	created_at: number;
	report_count: number; // For grouping
}

export interface AdminReportsResponse {
	reports: LinkReportWithLink[];
	pagination: {
		page: number;
		limit: number;
		total: number;
		pages: number;
	};
}

export interface UpdateReportStatusRequest {
	status: 'reviewed' | 'dismissed';
	admin_notes?: string;
}

export interface AdminLinksResponse {
	links: AdminLink[];
	total: number;
	page: number;
	limit: number;
}

// ─── Org Management Types ─────────────────────────────────────────────────────

export interface OrgWithRole {
	id: string;
	name: string;
	tier: string;
	role: 'owner' | 'member';
	joined_at: number;
}

export interface OrgMember {
	user_id: string;
	email: string;
	name: string | null;
	avatar_url: string | null;
	role: 'owner' | 'member';
	joined_at: number;
}

export interface OrgInvitation {
	id: string;
	org_id: string;
	invited_by: string;
	email: string;
	role: string;
	created_at: number;
	expires_at: number;
	accepted_at: number | null;
}

export interface OrgDetails {
	org: {
		id: string;
		name: string;
		tier: string;
		created_at: number;
		role: 'owner' | 'member';
	};
	members: OrgMember[];
	pending_invitations: OrgInvitation[];
}

export interface InviteInfo {
	valid: boolean;
	reason?: string;
	org_name?: string;
	invited_by?: string;
	email?: string;
	expires_at?: number;
}

export interface ListOrgsResponse {
	orgs: OrgWithRole[];
	current_org_id: string;
}

// ─── Billing Account Types ────────────────────────────────────────────────────

export interface BillingAccountWithStats {
	id: string;
	owner_user_id: string;
	owner_email: string;
	owner_name: string | null;
	tier: string;
	org_count: number;
	total_members: number;
	links_created_this_month: number;
	created_at: number;
}

export interface OrgWithMembersCount {
	id: string;
	name: string;
	slug: string;
	member_count: number;
	link_count: number;
	created_at: number;
}

export interface UsageStats {
	links_created_this_month: number;
	max_links_per_month: number | null;
	year_month: string;
}

export interface BillingAccountDetails {
	account: {
		id: string;
		owner_user_id: string;
		tier: string;
		created_at: number;
	};
	owner: User;
	organizations: OrgWithMembersCount[];
	usage: UsageStats;
}

export interface ListBillingAccountsResponse {
	accounts: BillingAccountWithStats[];
	total: number;
	page: number;
	limit: number;
}
