import { error } from "@sveltejs/kit";
import { COMPETITORS, COMPETITOR_SLUGS } from "../../../config/competitors";
import type { PageLoad } from "./$types";

export const prerender = true;
export const ssr = true;

export function entries() {
  return COMPETITOR_SLUGS.map((slug: string) => ({ competitor: slug }));
}

export const load: PageLoad = async ({ params }) => {
  const competitor = COMPETITORS[params.competitor];
  if (!competitor) {
    error(404, "Not found");
  }
  return { competitor };
};
