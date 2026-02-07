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
	is_active: boolean;
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
	is_active?: boolean;
}

export interface ApiError {
	message: string;
	status: number;
}
