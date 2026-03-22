import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';
import type { ApiError } from '$lib/types/api';

const API_BASE_URL = PUBLIC_VITE_API_BASE_URL || 'http://localhost:8787';
const TOKEN_KEY = 'rushomon_access_token';

// Store SvelteKit fetch function if available (for SSR/hydration compatibility)
let svelteKitFetch: typeof fetch | null = null;

export function setSvelteKitFetch(fetchFn: typeof fetch) {
	svelteKitFetch = fetchFn;
}

// Token management helpers
function getAccessToken(): string | null {
	if (typeof localStorage === 'undefined') {
		return null;
	}
	return localStorage.getItem(TOKEN_KEY);
}

function setAccessToken(token: string): void {
	if (typeof localStorage === 'undefined') {
		return;
	}
	localStorage.setItem(TOKEN_KEY, token);
}

function clearAccessToken(): void {
	if (typeof localStorage === 'undefined') {
		return;
	}
	localStorage.removeItem(TOKEN_KEY);
}

// Token refresh helper
async function refreshAccessToken(baseUrl: string): Promise<boolean> {
	try {
		const fetchFn = svelteKitFetch || fetch;
		const response = await fetchFn(`${baseUrl}/api/auth/refresh`, {
			method: 'POST',
			credentials: 'include',
		});

		if (!response.ok) {
			return false;
		}

		return true;
	} catch {
		return false;
	}
}

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
		const fetchFn = svelteKitFetch || fetch;

		const config: RequestInit = {
			...options,
			credentials: 'include',
			headers: {
				'Content-Type': 'application/json',
				...options.headers
			}
		};

		try {
			const response = await fetchFn(url, config);

			if (!response.ok) {
				if (response.status === 401) {
					const refreshed = await refreshAccessToken(this.baseUrl);

					if (refreshed) {
						const retryResponse = await fetchFn(url, config);
						if (retryResponse.ok) {
							const contentType = retryResponse.headers.get('content-type');
							if (contentType?.includes('application/json')) {
								return await retryResponse.json();
							}
							return {} as T;
						}
					}
				}

				const contentType = response.headers.get('content-type');
				let errorMessage: string;

				if (contentType?.includes('application/json')) {
					const errorData = await response.json();
					errorMessage = errorData.message || 'An error occurred';

					const error: ApiError = {
						message: errorMessage,
						status: response.status,
						data: errorData // Preserve full error data
					};
					throw error;
				} else {
					errorMessage = await response.text();

					const error: ApiError = {
						message: errorMessage,
						status: response.status
					};
					throw error;
				}
			}

			const contentType = response.headers.get('content-type');
			if (contentType?.includes('application/json')) {
				return await response.json();
			}

			return {} as T;
		} catch (error) {
			if (error && typeof error === 'object' && 'status' in error) {
				throw error;
			}

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

	async patch<T>(endpoint: string, data?: unknown): Promise<T> {
		return this.request<T>(endpoint, {
			method: 'PATCH',
			body: data ? JSON.stringify(data) : undefined
		});
	}

	async delete<T>(endpoint: string, data?: unknown): Promise<T> {
		return this.request<T>(endpoint, {
			method: 'DELETE',
			body: data ? JSON.stringify(data) : undefined
		});
	}

	async postForm<T>(endpoint: string, formData: FormData): Promise<T> {
		const url = `${this.baseUrl}${endpoint}`;
		const fetchFn = svelteKitFetch || fetch;
		const config: RequestInit = {
			method: 'POST',
			credentials: 'include',
			body: formData,
			// No Content-Type header: browser will set multipart/form-data with boundary
		};
		const response = await fetchFn(url, config);
		if (!response.ok) {
			const contentType = response.headers.get('content-type');
			let errorMessage: string;
			if (contentType?.includes('application/json')) {
				const errorData = await response.json();
				errorMessage = errorData.message || 'An error occurred';
				throw { message: errorMessage, status: response.status, data: errorData };
			} else {
				errorMessage = await response.text();
				throw { message: errorMessage, status: response.status };
			}
		}
		const contentType = response.headers.get('content-type');
		if (contentType?.includes('application/json')) {
			return await response.json();
		}
		return {} as T;
	}

	async fetchRaw(endpoint: string, options: RequestInit = {}): Promise<Response> {
		const url = `${this.baseUrl}${endpoint}`;
		const fetchFn = svelteKitFetch || fetch;
		const config: RequestInit = {
			...options,
			credentials: 'include',
			headers: {
				'Content-Type': 'application/json',
				...options.headers
			}
		};
		const response = await fetchFn(url, config);
		if (!response.ok) {
			const errorMessage = await response.text();
			throw { message: errorMessage, status: response.status };
		}
		return response;
	}
}

export const apiClient = new ApiClient();

export { getAccessToken, setAccessToken, clearAccessToken };

/**
 * Resolve a relative logo URL (e.g. /api/orgs/:id/logo) to an absolute URL
 * pointing at the API server. Required because logo_url is stored as a relative
 * path but the frontend is served from a different origin in development.
 */
export function resolveLogoUrl(logoUrl: string | null | undefined): string | null {
	if (!logoUrl) return null;
	if (logoUrl.startsWith('http://') || logoUrl.startsWith('https://')) return logoUrl;
	return `${API_BASE_URL}${logoUrl}`;
}
