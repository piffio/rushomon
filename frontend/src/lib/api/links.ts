import { apiClient } from './client';
import type { Link, CreateLinkRequest, UpdateLinkRequest, PaginatedResponse, LinkAnalyticsResponse, TagWithCount } from '$lib/types/api';

export const linksApi = {
	/**
	 * List all links for the authenticated user's organization
	 * @param page - Page number (default: 1)
	 * @param limit - Number of links per page (default: 20)
	 * @param search - Search term for filtering links (optional)
	 * @param status - Status filter: 'active', 'disabled', or undefined (optional)
	 * @param sort - Sort order: 'created', 'updated', 'clicks', 'title', 'code' (default: 'created')
	 * @returns Paginated response with links and pagination metadata
	 */
	async list(
		page: number = 1,
		limit: number = 20,
		search?: string,
		status?: 'active' | 'disabled',
		sort?: 'created' | 'updated' | 'clicks' | 'title' | 'code',
		tags?: string[]
	): Promise<PaginatedResponse<Link>> {
		const params = new URLSearchParams();
		params.set('page', page.toString());
		params.set('limit', limit.toString());
		if (search?.trim()) {
			params.set('search', search.trim());
		}
		if (status) {
			params.set('status', status);
		}
		if (sort && sort !== 'created') {
			params.set('sort', sort);
		}
		if (tags && tags.length > 0) {
			params.set('tags', tags.join(','));
		}
		return apiClient.get<PaginatedResponse<Link>>(`/api/links?${params.toString()}`);
	},

	/**
	 * Get a single link by ID
	 * @param id - Link UUID
	 * @returns Link object
	 * @throws ApiError if link not found (404)
	 */
	async get(id: string): Promise<Link> {
		return apiClient.get<Link>(`/api/links/${id}`);
	},

	/**
	 * Create a new short link
	 * @param data - Link creation data
	 * @returns Created Link object
	 * @throws ApiError if validation fails (400) or short code taken (409)
	 */
	async create(data: CreateLinkRequest): Promise<Link> {
		return apiClient.post<Link>('/api/links', data);
	},

	/**
	 * Update a link
	 * @param id - Link UUID
	 * @param data - Update data (only provided fields will be updated)
	 * @returns Updated Link object
	 * @throws ApiError if link not found (404) or validation fails (400)
	 */
	async update(id: string, data: UpdateLinkRequest): Promise<Link> {
		return apiClient.request<Link>(`/api/links/${id}`, {
			method: 'PUT',
			body: JSON.stringify(data)
		});
	},

	/**
	 * Delete a link
	 * Soft deletes from D1, hard deletes from KV (stops redirects immediately)
	 * @param id - Link UUID
	 * @throws ApiError if link not found (404)
	 */
	async delete(id: string): Promise<void> {
		await apiClient.delete<void>(`/api/links/${id}`);
	},

	/**
	 * Get a link by its short_code
	 * @param shortCode - The short code of the link
	 * @returns Link object
	 * @throws ApiError if link not found (404)
	 */
	async getByCode(shortCode: string): Promise<Link> {
		return apiClient.get<Link>(`/api/links/by-code/${shortCode}`);
	},

	/**
	 * Get analytics data for a link
	 * @param id - Link UUID
	 * @param days - Number of days to analyze (e.g., 7 for last 7 days, 0 for all time)
	 * @returns Analytics response with clicks over time, referrers, countries, user agents
	 */
	async getAnalytics(id: string, days: number): Promise<LinkAnalyticsResponse> {
		// Backend now calculates timestamps to eliminate clock skew issues
		return apiClient.get<LinkAnalyticsResponse>(`/api/links/${id}/analytics?days=${days}`);
	}
};

export const tagsApi = {
	/**
	 * Get all tags for the authenticated org with usage counts
	 * @returns Array of tags sorted by usage count desc
	 */
	async list(): Promise<TagWithCount[]> {
		return apiClient.get<TagWithCount[]>('/api/tags');
	}
};
