import { apiClient } from './client';

export interface BillingStatus {
	tier: string;
	is_billing_owner: boolean;
	interval: string | null;
	amount_cents: number | null;
	currency: string | null;
	discount_name: string | null;
	provider_customer_id: string | null;
	billing_account_id: string | null;
	subscription_id: string | null;
	subscription_status: string | null;
	subscription_plan: string | null;
	current_period_end: number | null;
	cancel_at_period_end: boolean;
}

export interface CheckoutRequest {
	price_id: string;
	billing_interval: 'monthly' | 'annual';
	coupon_id?: string;
}

export interface CheckoutResponse {
	url: string;
}

export interface PortalResponse {
	url: string;
}

export interface ProductPrice {
	id: string;
	polar_product_id: string;
	polar_price_id: string;
	name: string;
	price_amount: number;
	price_currency: string;
	recurring_interval: string | null;
	recurring_interval_count: number | null;
}

export interface PricingResponse {
	products: ProductPrice[];
}

export const billingApi = {
	getStatus(): Promise<BillingStatus> {
		return apiClient.get<BillingStatus>('/api/billing/status');
	},

	getPricing(): Promise<PricingResponse> {
		return apiClient.get<PricingResponse>('/api/billing/pricing');
	},

	async createCheckout(price_key: string, billing_interval: 'monthly' | 'annual', coupon_id?: string): Promise<CheckoutResponse> {
		const requestBody: CheckoutRequest = {
			price_id: price_key,
			billing_interval
		};

		if (coupon_id) {
			requestBody.coupon_id = coupon_id;
		}

		const res = await apiClient.post<CheckoutResponse>('/api/billing/checkout', requestBody);

		if (!res.url) {
			throw new Error('No checkout URL returned');
		}

		return { url: res.url };
	},

	async createPortal(): Promise<PortalResponse> {
		const response = await fetch('/api/billing/portal', {
			method: 'POST',
			credentials: 'include',
			headers: {
				'Content-Type': 'application/json',
			},
		});

		if (!response.ok) {
			const error = await response.text();
			throw new Error(error || 'Failed to create portal session');
		}

		return response.json() as Promise<PortalResponse>;
	}
};
