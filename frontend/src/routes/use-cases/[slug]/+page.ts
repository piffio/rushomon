import { USE_CASES, USE_CASE_SLUGS } from "../../../config/use-cases";
import { error } from "@sveltejs/kit";
import type { PageLoad } from "./$types";

export const prerender = true;
export const ssr = true;

export function entries() {
  return USE_CASE_SLUGS.map((slug: string) => ({ slug }));
}

export const load: PageLoad = async ({ params }) => {
  const useCase = USE_CASES[params.slug];
  if (!useCase) {
    error(404, "Not found");
  }
  return { useCase };
};
