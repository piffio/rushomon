import type { PageLoad } from './$types';

export const load: PageLoad = async ({ parent }) => {
	// Get user data from layout (client-side)
	const parentData = await parent() as { user?: any; };
	const user = parentData.user;

	return {
		user,
	};
};
