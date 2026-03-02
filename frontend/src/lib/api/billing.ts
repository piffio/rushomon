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
}

export interface CheckoutResponse {
	url: string;
}

export interface PortalResponse {
	url: string;
}

export const billingApi = {
	getStatus(): Promise<BillingStatus> {
		return apiClient.get<BillingStatus>('/api/billing/status');
	},

	async createCheckout(price_key: string, billing_interval: 'monthly' | 'annual'): Promise<CheckoutResponse> {
		const res = await apiClient.post<CheckoutResponse>('/api/billing/checkout', {
			price_id: price_key
		});

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
