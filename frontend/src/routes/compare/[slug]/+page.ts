import { error } from "@sveltejs/kit";
import { COMPETITORS, COMPETITOR_SLUGS } from "../../../config/competitors";
import type { PageLoad } from "./$types";

export const prerender = true;
export const ssr = true;

export function entries() {
  return COMPETITOR_SLUGS.map((slug: string) => ({
    slug: `rushomon-vs-${slug}`
  }));
}

export const load: PageLoad = async ({ params }) => {
  const match = params.slug.match(/^rushomon-vs-(.+)$/);
  if (!match) {
    error(404, "Not found");
  }
  const competitor = COMPETITORS[match[1]];
  if (!competitor) {
    error(404, "Not found");
  }
  return { competitor };
};
