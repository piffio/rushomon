export interface PublicPage {
	path: string;
	changefreq: string;
	priority: string;
	indexable?: boolean;
}

export const PUBLIC_PAGES: PublicPage[] = [
	{ path: '/', changefreq: 'weekly', priority: '1.0', indexable: true },
	{ path: '/pricing', changefreq: 'monthly', priority: '0.8', indexable: true },
	{ path: '/report', changefreq: 'monthly', priority: '0.6', indexable: true },
	{ path: '/terms', changefreq: 'yearly', priority: '0.3', indexable: true },
	{ path: '/privacy', changefreq: 'yearly', priority: '0.3', indexable: true }
];
