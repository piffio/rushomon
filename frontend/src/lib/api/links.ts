import { apiClient } from './client';
import type { Link, CreateLinkRequest, UpdateLinkRequest } from '$lib/types/api';

export const linksApi = {
	/**
	 * List all links for the authenticated user's organization
	 * @param page - Page number (default: 1)
	 * @param limit - Number of links per page (default: 20)
	 * @returns Array of Link objects
	 */
	async list(page: number = 1, limit: number = 20): Promise<Link[]> {
		return apiClient.get<Link[]>(`/api/links?page=${page}&limit=${limit}`);
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
	}
};
