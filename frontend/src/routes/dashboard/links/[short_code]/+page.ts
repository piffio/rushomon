import { linksApi } from "$lib/api/links";
import { usageApi } from "$lib/api/usage";
import type {
  Link,
  LinkAnalyticsResponse,
  UsageResponse
} from "$lib/types/api";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ params, parent, url }) => {
  const parentData = (await parent()) as { user?: unknown };
  const user = parentData.user;

  if (!user) {
    return { user: null, link: null, analytics: null, error: null, tier: null };
  }

  try {
    // Look up link by short_code
    const link: Link = await linksApi.getByCode(params.short_code);

    // Parse time range from URL query params (or use defaults)
    const days = parseInt(url.searchParams.get("days") || "7", 10);

    // Fetch analytics and usage (tier info) in parallel
    // Backend now calculates timestamps to eliminate clock skew issues
    const [analytics, usage]: [LinkAnalyticsResponse, UsageResponse | null] =
      await Promise.all([
        linksApi.getAnalytics(link.id, days),
        usageApi.getUsage().catch(() => null)
      ]);

    return {
      user,
      link,
      analytics,
      days,
      error: null,
      tier: usage?.tier || "free"
    };
  } catch (error: unknown) {
    console.error("Failed to load link analytics:", error);
    return {
      user,
      link: null,
      analytics: null,
      days: 7,
      error:
        (error instanceof Error ? error.message : String(error)) ||
        "Failed to load analytics",
      tier: "free"
    };
  }
};
