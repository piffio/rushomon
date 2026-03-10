import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ parent }) => {
	const parentData = await parent() as { user?: any };
	return { user: parentData.user ?? null };
};
