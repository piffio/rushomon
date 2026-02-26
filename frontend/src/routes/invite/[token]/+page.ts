export const load = async ({
	params,
	parent,
}: {
	params: { token: string };
	parent: () => Promise<{ user?: unknown }>;
}) => {
	const parentData = await parent();
	return {
		token: params.token,
		user: (parentData.user ?? null) as import('$lib/types/api').User | null,
	};
};
