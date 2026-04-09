import type { User } from "$lib/types/api";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ parent }) => {
  const parentData = (await parent()) as { user?: User };
  return {
    user: parentData.user
  };
};
