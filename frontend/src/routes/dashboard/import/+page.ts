import { usageApi } from "$lib/api/usage";

export const load = async ({
  parent
}: {
  parent: () => Promise<{ user?: unknown }>;
}) => {
  const parentData = await parent();
  const user = parentData.user;

  const usage = await usageApi.getUsage().catch(() => null);

  return { user, usage };
};
