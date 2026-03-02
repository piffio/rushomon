import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';

// Price ID mapping from human-readable keys to UUIDs
// These map to the price IDs configured in the worker via environment variables
const PRICE_KEY_MAP: Record<string, string> = {
	'pro_monthly': 'pro_monthly',
	'pro_annual': 'pro_annual',
	'business_monthly': 'business_monthly',
	'business_annual': 'business_annual'
};

export const POST: RequestHandler = async (event) => {
	const jwtToken = event.cookies.get('rushomon_access');
	if (!jwtToken) {
		throw error(401, 'Not authenticated');
	}

	const body = await event.request.json().catch(() => ({}));
	const products = body.products as string | undefined;

	if (!products || !PRICE_KEY_MAP[products]) {
		throw error(400, `Invalid products key. Valid keys are: ${Object.keys(PRICE_KEY_MAP).join(', ')}`);
	}

	const workerBase = PUBLIC_VITE_API_BASE_URL;

	// Proxy the checkout request to the worker, which handles:
	// 1. Authentication
	// 2. Billing account lookup
	// 3. Polar API call with customer_external_id = billing_account_id
	const workerRes = await fetch(`${workerBase}/api/billing/checkout`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			'Authorization': `Bearer ${jwtToken}`
		},
		body: JSON.stringify({ price_id: products })
	});

	if (!workerRes.ok) {
		const errBody = await workerRes.text().catch(() => 'Unknown error');
		return Response.json(
			{ error: errBody || 'Failed to create checkout session' },
			{ status: workerRes.status }
		);
	}

	const data = await workerRes.json();
	return Response.json(data);
};
