export interface PricingTier {
	tier: string;
	title: string;
	description: string;
	price: number | (() => number);
	interval: string;
	features: string[];
	buttonText: string | (() => string);
	buttonHref?: string | (() => string | undefined);
	isPopular: boolean;
	founderPrice?: () => number;
	originalPrice?: () => number;
	disabled?: boolean | (() => boolean);
}

// Helper function to get actual price from database (not fallback)
function getActualPrice(
	plan: "pro" | "business",
	interval: "monthly" | "annual",
	products: any[]
): number {
	const settingKey = `product_${plan}_${interval}_id`;
	const product = products.find((p: any) => p.id === settingKey);
	return product?.price_amount || 0;
}

// Helper function to determine tier hierarchy
function getTierHierarchy(tier: string): number {
	switch (tier) {
		case 'free': return 0;
		case 'pro': return 1;
		case 'business': return 2;
		default: return 0;
	}
}

// Helper function to check if a tier is the current plan
function isCurrentPlan(tier: string, billingStatus: any): boolean {
	if (!billingStatus || !billingStatus.tier) return false;
	return billingStatus.tier.toLowerCase() === tier.toLowerCase();
}

// Helper function to check if a tier is an upgrade
function isUpgrade(tier: string, billingStatus: any): boolean {
	if (!billingStatus || !billingStatus.tier) return true; // No current plan, everything is an upgrade
	const currentTier = getTierHierarchy(billingStatus.tier.toLowerCase());
	const targetTier = getTierHierarchy(tier.toLowerCase());
	return targetTier > currentTier;
}

// Helper function to check if a tier is a downgrade
function isDowngrade(tier: string, billingStatus: any): boolean {
	if (!billingStatus || !billingStatus.tier) return false;
	const currentTier = getTierHierarchy(billingStatus.tier.toLowerCase());
	const targetTier = getTierHierarchy(tier.toLowerCase());
	return targetTier < currentTier;
}

// Helper function to check if user is on free plan
function isOnFreePlan(billingStatus: any): boolean {
	return !billingStatus || billingStatus.tier?.toLowerCase() === 'free';
}

export const createPricingTiers = (
	getDisplayPriceWithFallback: (tier: string, interval: string) => number,
	founderPrice: (tier: string, interval: string) => number,
	billingInterval: string,
	currentUser: any,
	loginUrl: string,
	products: any[],
	billingStatus: any,
	openPortal: () => void
): PricingTier[] => [
		{
			tier: 'free',
			title: 'Free',
			description: 'Perfect for individuals getting started',
			price: 0,
			interval: 'month',
			features: [
				'15 links per month',
				'7-day analytics retention',
				'rush.mn short links',
				'QR code generation',
				'Community support (GitHub)'
			],
			buttonText: () => {
				if (!currentUser) return 'Get Started Free';
				if (isCurrentPlan('free', billingStatus)) return 'Your Current Plan';
				if (isDowngrade('free', billingStatus)) return 'Keep Current Plan';
				return 'Downgrade to Free';
			},
			buttonHref: () => {
				if (!currentUser) return loginUrl;
				if (isCurrentPlan('free', billingStatus)) return undefined; // Use portal action
				if (isDowngrade('free', billingStatus)) return '/dashboard'; // Redirect to dashboard
				return undefined; // Downgrades not implemented yet
			},
			disabled: false, // Free tier is never disabled
			isPopular: false
		},
		{
			tier: 'pro',
			title: 'Pro',
			description: 'For creators who need custom codes, extended analytics, and advanced redirect controls',
			price: () => getDisplayPriceWithFallback('pro', billingInterval),
			interval: billingInterval === 'monthly' ? 'month' : 'year',
			features: [
				'Everything in Free',
				'1,000 links per month',
				'1-year analytics retention',
				'Custom short codes',
				'Advanced QR codes (sizes, SVG, org logo)',
				'Redirect type selection (301/307)',
				'Email support'
			],
			buttonText: () => {
				const actualPrice = getActualPrice('pro', billingInterval as "monthly" | "annual", products);
				// If actual price is 0, plan isn't configured
				if (actualPrice === 0) return 'Coming Soon';

				if (!currentUser) return 'Get Started';
				if (isCurrentPlan('pro', billingStatus)) return 'Manage Subscription';
				if (isDowngrade('pro', billingStatus)) return 'Keep Current Plan';
				if (isUpgrade('pro', billingStatus)) {
					return isOnFreePlan(billingStatus) ? 'Upgrade to Pro' : 'Upgrade to Pro';
				}
				return 'Get Started';
			},
			buttonHref: () => {
				const actualPrice = getActualPrice('pro', billingInterval as "monthly" | "annual", products);
				// If actual price is 0, plan isn't configured, disable button
				if (actualPrice === 0) return undefined;

				if (!currentUser) return loginUrl;
				if (isCurrentPlan('pro', billingStatus)) return undefined; // Use portal action
				if (isDowngrade('pro', billingStatus)) return '/dashboard'; // Redirect to dashboard
				if (isUpgrade('pro', billingStatus)) {
					// Free users use checkout, paid users use portal
					return isOnFreePlan(billingStatus) ? undefined : undefined;
				}
				return undefined; // Use checkout action
			},
			disabled: () => {
				const actualPrice = getActualPrice('pro', billingInterval as "monthly" | "annual", products);
				// Disabled when plan is not configured (actualPrice === 0)
				return actualPrice === 0;
			},
			isPopular: true,
			founderPrice: () => founderPrice('pro', billingInterval),
			originalPrice: () => getDisplayPriceWithFallback('pro', billingInterval)
		},
		{
			tier: 'business',
			title: 'Business',
			description: 'For teams needing long-term analytics insights',
			price: () => getDisplayPriceWithFallback('business', billingInterval),
			interval: billingInterval === 'monthly' ? 'month' : 'year',
			features: [
				'Everything in Pro',
				'10,000 links per month',
				'3-year analytics retention',
				'3 organizations',
				'20 team members',
				'Device-based routing',
				'Password protection',
				'Priority support'
			],
			buttonText: () => {
				const actualPrice = getActualPrice('business', billingInterval as "monthly" | "annual", products);
				// If actual price is 0, plan isn't configured
				if (actualPrice === 0) return 'Coming Soon';

				if (!currentUser) return 'Get Started';
				if (isCurrentPlan('business', billingStatus)) return 'Manage Subscription';
				if (isDowngrade('business', billingStatus)) return 'Keep Current Plan';
				if (isUpgrade('business', billingStatus)) {
					return isOnFreePlan(billingStatus) ? 'Upgrade to Business' : 'Upgrade to Business';
				}
				return 'Get Started';
			},
			buttonHref: () => {
				const actualPrice = getActualPrice('business', billingInterval as "monthly" | "annual", products);
				// If actual price is 0, plan isn't configured, disable button
				if (actualPrice === 0) return undefined;

				if (!currentUser) return loginUrl;
				if (isCurrentPlan('business', billingStatus)) return undefined; // Use portal action
				if (isDowngrade('business', billingStatus)) return '/dashboard'; // Redirect to dashboard
				if (isUpgrade('business', billingStatus)) {
					// Free users use checkout, paid users use portal
					return isOnFreePlan(billingStatus) ? undefined : undefined;
				}
				return undefined; // Use checkout action
			},
			disabled: () => {
				const actualPrice = getActualPrice('business', billingInterval as "monthly" | "annual", products);
				// Disabled when plan is not configured (actualPrice === 0)
				return actualPrice === 0;
			},
			isPopular: false,
			founderPrice: () => founderPrice('business', billingInterval),
			originalPrice: () => getDisplayPriceWithFallback('business', billingInterval)
		}
	];
