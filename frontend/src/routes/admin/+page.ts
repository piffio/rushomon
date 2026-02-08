import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { adminApi } from '$lib/api/admin';

export const load: PageLoad = async ({ parent }) => {
	const parentData = (await parent()) as { user?: any };
	const user = parentData.user;

	if (!user) {
		throw redirect(302, '/');
	}

	// Only admins can access the admin dashboard
	if (user.role !== 'admin') {
		throw redirect(302, '/dashboard');
	}

	try {
		const usersResponse = await adminApi.listUsers(1, 50);

		return {
			user,
			users: usersResponse.users || [],
			total: usersResponse.total || 0
		};
	} catch (error) {
		console.error('Failed to fetch users:', error);
		return {
			user,
			users: [],
			total: 0
		};
	}
};
