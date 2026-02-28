export const load = async ({ parent }: { parent: () => Promise<{ user?: unknown; }>; }) => {
	const parentData = await parent();
	return { user: (parentData.user ?? null) as import('$lib/types/api').User | null };
};
