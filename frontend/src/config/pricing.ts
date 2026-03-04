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

export const createPricingTiers = (
	getDisplayPriceWithFallback: (tier: string, interval: string) => number,
	founderPrice: (tier: string, interval: string) => number,
	billingInterval: string,
	currentUser: any,
	loginUrl: string,
	products: any[]
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
			buttonText: 'Get Started Free',
			buttonHref: loginUrl,
			isPopular: false,
			disabled: false
		},
		{
			tier: 'pro',
			title: 'Pro',
			description: 'For creators who need custom codes and extended analytics history',
			price: () => getDisplayPriceWithFallback('pro', billingInterval),
			interval: billingInterval === 'monthly' ? 'month' : 'year',
			features: [
				'Everything in Free',
				'1,000 links per month',
				'1-year analytics retention',
				'Custom short codes',
				'Email support'
			],
			buttonText: () => {
				const actualPrice = getActualPrice('pro', billingInterval as "monthly" | "annual", products);
				// If actual price is 0, plan isn't configured
				return actualPrice === 0 ? 'Coming Soon' : (currentUser ? 'Upgrade to Pro' : 'Get Started');
			},
			buttonHref: () => {
				const actualPrice = getActualPrice('pro', billingInterval as "monthly" | "annual", products);
				// If actual price is 0, plan isn't configured, disable button
				return actualPrice === 0 ? undefined : (currentUser ? undefined : loginUrl);
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
				return actualPrice === 0 ? 'Coming Soon' : (currentUser ? 'Upgrade to Business' : 'Get Started');
			},
			buttonHref: () => {
				const actualPrice = getActualPrice('business', billingInterval as "monthly" | "annual", products);
				// If actual price is 0, plan isn't configured, disable button
				return actualPrice === 0 ? undefined : (currentUser ? undefined : loginUrl);
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
