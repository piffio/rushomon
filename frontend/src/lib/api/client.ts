import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';
import type { ApiError } from '$lib/types/api';

const API_BASE_URL = PUBLIC_VITE_API_BASE_URL || 'http://localhost:8787';
const TOKEN_KEY = 'rushomon_access_token';

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
		const response = await fetch(`${baseUrl}/api/auth/refresh`, {
			method: 'POST',
			credentials: 'include', // Send refresh token cookie
		});

		if (!response.ok) {
			return false;
		}

		const data = await response.json();
		setAccessToken(data.access_token);
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
		const token = getAccessToken();

		const config: RequestInit = {
			...options,
			credentials: 'include', // Include cookies (for refresh token and backward compatibility)
			headers: {
				'Content-Type': 'application/json',
				...(token ? { 'Authorization': `Bearer ${token}` } : {}),
				...options.headers
			}
		};

		try {
			const response = await fetch(url, config);

			// Handle non-JSON error responses (plain text)
			if (!response.ok) {
				// If 401 and we have a token, try to refresh
				if (response.status === 401 && token) {
					const refreshed = await refreshAccessToken(this.baseUrl);

					if (refreshed) {
						// Retry request with new token
						const newToken = getAccessToken();
						const retryConfig = {
							...config,
							headers: {
								...config.headers,
								'Authorization': `Bearer ${newToken}`
							}
						};

						const retryResponse = await fetch(url, retryConfig);
						if (retryResponse.ok) {
							const contentType = retryResponse.headers.get('content-type');
							if (contentType?.includes('application/json')) {
								return await retryResponse.json();
							}
							return {} as T;
						}
					}

					// Refresh failed - clear token and throw
					clearAccessToken();
				}

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

// Export token management helpers for use in other modules
export { getAccessToken, setAccessToken, clearAccessToken };
