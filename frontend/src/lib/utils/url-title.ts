import { apiClient } from '$lib/api/client';

interface FetchTitleResponse {
	title: string | null;
}

/**
 * Fetches the title of a web page from a given URL via backend API
 * @param url - The URL to fetch the title from
 * @returns Promise<string | null> - The page title or null if unable to fetch
 */
export async function fetchUrlTitle(url: string): Promise<string | null> {
	// Basic URL validation - must start with http:// or https://
	if (!url || typeof url !== 'string') {
		return null;
	}

	const trimmedUrl = url.trim();
	if (!trimmedUrl.startsWith('http://') && !trimmedUrl.startsWith('https://')) {
		return null;
	}

	try {
		const response = await apiClient.post<FetchTitleResponse>('/api/fetch-title', { url: trimmedUrl });
		return response.title || null;
	} catch (error) {
		// Silently handle errors - title fetching is optional
		console.debug('Failed to fetch URL title:', error);
		return null;
	}
}

/**
 * Debounce function to limit how often a function is called
 * @param func - The function to debounce
 * @param delay - Delay in milliseconds
 * @returns Debounced function
 */
export function debounce<T extends (...args: any[]) => any>(
	func: T,
	delay: number
): (...args: Parameters<T>) => void {
	let timeoutId: number;

	return (...args: Parameters<T>) => {
		clearTimeout(timeoutId);
		timeoutId = setTimeout(() => func(...args), delay);
	};
}
