import type { User } from "$lib/types/api";
import type { LayoutLoad } from "./$types";

export const load: LayoutLoad = async ({ parent }) => {
  const parentData = (await parent()) as { user?: User };
  return { user: parentData.user ?? null };
};
