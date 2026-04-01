import { analyticsApi } from "$lib/api/analytics";
import { usageApi } from "$lib/api/usage";
import type { OrgAnalyticsResponse, UsageResponse } from "$lib/types/api";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ parent, url }) => {
  await parent();

  const days = parseInt(url.searchParams.get("days") || "7", 10);
  const startParam = url.searchParams.get("start");
  const endParam = url.searchParams.get("end");

  const isCustomRange = startParam !== null && endParam !== null;

  const [analytics, usage]: [
    OrgAnalyticsResponse | null,
    UsageResponse | null
  ] = await Promise.all([
    isCustomRange
      ? analyticsApi.getOrgAnalyticsCustomRange(
          parseInt(startParam!),
          parseInt(endParam!)
        )
      : analyticsApi.getOrgAnalytics(days),
    usageApi.getUsage().catch(() => null)
  ]);

  return {
    analytics,
    days,
    startParam,
    endParam,
    isCustomRange,
    tier: (usage?.tier as string) || "free"
  };
};
