export interface PricingTier {
	tier: string;
	title: string;
	description: string;
	price: number | (() => number);
	interval: string;
	features: string[];
	buttonText: string;
	buttonHref?: string;
	isPopular: boolean;
	founderPrice?: () => number;
	originalPrice?: () => number;
}

export const createPricingTiers = (
	getDisplayPriceWithFallback: (tier: string, interval: string) => number,
	founderPrice: (tier: string, interval: string) => number,
	billingInterval: string,
	currentUser: any,
	loginUrl: string
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
			isPopular: false
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
			buttonText: currentUser ? 'Upgrade to Pro' : 'Get Started',
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
			buttonText: currentUser ? 'Upgrade to Business' : 'Get Started',
			isPopular: false,
			founderPrice: () => founderPrice('business', billingInterval),
			originalPrice: () => getDisplayPriceWithFallback('business', billingInterval)
		}
	];
