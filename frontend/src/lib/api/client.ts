import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';
import type { ApiError } from '$lib/types/api';

const API_BASE_URL = PUBLIC_VITE_API_BASE_URL || 'http://localhost:8787';

export class ApiClient {
	private baseUrl: string;

	constructor(baseUrl: string = API_BASE_URL) {
		this.baseUrl = baseUrl;
	}

	async request<T>(
		endpoint: string,
		options: RequestInit = {}
	): Promise<T> {
		const url = `${this.baseUrl}${endpoint}`;

		const config: RequestInit = {
			...options,
			credentials: 'include', // Include cookies for authentication
			headers: {
				'Content-Type': 'application/json',
				...options.headers
			}
		};

		try {
			const response = await fetch(url, config);

			// Handle non-JSON error responses (plain text)
			if (!response.ok) {
				const contentType = response.headers.get('content-type');
				let errorMessage: string;

				if (contentType?.includes('application/json')) {
					const errorData = await response.json();
					errorMessage = errorData.message || 'An error occurred';
				} else {
					errorMessage = await response.text();
				}

				const error: ApiError = {
					message: errorMessage,
					status: response.status
				};
				throw error;
			}

			// Handle empty responses
			const contentType = response.headers.get('content-type');
			if (contentType?.includes('application/json')) {
				return await response.json();
			}

			// For empty responses (like DELETE), return empty object
			return {} as T;
		} catch (error) {
			// Re-throw ApiError as-is
			if (error && typeof error === 'object' && 'status' in error) {
				throw error;
			}

			// Network or other errors
			throw {
				message: error instanceof Error ? error.message : 'Network error',
				status: 0
			} as ApiError;
		}
	}

	async get<T>(endpoint: string): Promise<T> {
		return this.request<T>(endpoint, { method: 'GET' });
	}

	async post<T>(endpoint: string, data?: unknown): Promise<T> {
		return this.request<T>(endpoint, {
			method: 'POST',
			body: data ? JSON.stringify(data) : undefined
		});
	}

	async delete<T>(endpoint: string): Promise<T> {
		return this.request<T>(endpoint, { method: 'DELETE' });
	}
}

export const apiClient = new ApiClient();
