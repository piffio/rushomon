import { apiClient } from "./client";

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
  if (!url || typeof url !== "string") {
    return null;
  }

  const trimmedUrl = url.trim();
  if (!trimmedUrl.startsWith("http://") && !trimmedUrl.startsWith("https://")) {
    return null;
  }

  try {
    const response = await apiClient.post<FetchTitleResponse>(
      "/api/fetch-title",
      { url: trimmedUrl }
    );
    return response.title || null;
  } catch (error) {
    // Silently handle errors - title fetching is optional
    console.debug("Failed to fetch URL title:", error);
    return null;
  }
}
