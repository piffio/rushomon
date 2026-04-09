import { linksApi } from "$lib/api/links";
import { orgsApi } from "$lib/api/orgs";
import { usageApi } from "$lib/api/usage";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ parent, url, depends }) => {
  // Declare dependency for invalidation
  depends("app:dashboard");

  // Get user data from layout (now client-side)
  const parentData = (await parent()) as { user?: any };
  const user = parentData.user;

  if (!user) {
    // This shouldn't happen if layout is working, but just in case
    return {
      user: null,
      paginatedLinks: null,
      usage: null,
      initialSearch: "",
      initialStatus: "all",
      initialSort: "created"
    };
  }

  try {
    // Get params from URL query params
    const page = parseInt(url.searchParams.get("page") || "1", 10);
    const search = url.searchParams.get("search") || "";
    const status = url.searchParams.get("status") as
      | "active"
      | "disabled"
      | null;
    const sort = (url.searchParams.get("sort") || "created") as
      | "created"
      | "updated"
      | "clicks"
      | "title"
      | "code";
    const tags = (url.searchParams.get("tags") || "")
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);

    // Fetch links, usage, and org details in parallel
    const [paginatedLinks, usage, orgId, orgLogoUrl] = await Promise.all([
      linksApi.list(
        page,
        10,
        search || undefined,
        status || undefined,
        sort,
        tags.length > 0 ? tags : undefined
      ),
      usageApi.getUsage().catch(() => null),
      orgsApi
        .listMyOrgs()
        .then((r) => r.current_org_id)
        .catch(() => ""),
      orgsApi
        .listMyOrgs()
        .then((r) => orgsApi.getOrg(r.current_org_id))
        .then((d) => d.org.logo_url)
        .catch(() => null)
    ]);

    return {
      user,
      paginatedLinks,
      usage,
      orgLogoUrl,
      orgId,
      initialSearch: search,
      initialStatus: status || "all",
      initialSort: sort,
      initialTags: tags
    };
  } catch (error) {
    // If links fetch fails, still return user data
    console.error("Failed to fetch links:", error);
    return {
      user,
      paginatedLinks: null,
      usage: null,
      orgLogoUrl: null,
      orgId: "",
      initialSearch: "",
      initialStatus: "all",
      initialSort: "created"
    };
  }
};
