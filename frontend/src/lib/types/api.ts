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
}

export type LinkStatus = 'active' | 'disabled';

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
}

export interface CreateLinkRequest {
	destination_url: string;
	short_code?: string;
	title?: string;
	expires_at?: number;
}

export interface UpdateLinkRequest {
	destination_url?: string;
	title?: string;
	expires_at?: number;
	status?: LinkStatus;
}

export interface ApiError {
	message: string;
	status: number;
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
	};
	usage: {
		links_created_this_month: number;
	};
}
