import { error } from '@sveltejs/kit';
import { CustomerPortal } from '@polar-sh/sveltekit';
import type { RequestHandler } from './$types';
import { POLAR_ACCESS_TOKEN, POLAR_SANDBOX } from '$env/static/private';
import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';

export const POST: RequestHandler = async (event) => {
	// Force a visible error to confirm endpoint is reached
	console.error('PORTAL ENDPOINT REACHED - DEBUG');

	if (!POLAR_ACCESS_TOKEN) {
		return error(503, 'Billing not configured - no POLAR_ACCESS_TOKEN');
	}

	const jwtToken = event.cookies.get('rushomon_access');
	if (!jwtToken) {
		console.error('No JWT token found');
		throw error(401, 'Not authenticated');
	}

	const workerBase = PUBLIC_VITE_API_BASE_URL;
	console.log('Fetching billing status from:', `${workerBase}/api/billing/status`);

	const statusRes = await fetch(`${workerBase}/api/billing/status`, {
		headers: { Authorization: `Bearer ${jwtToken}` }
	});

	if (!statusRes.ok) {
		console.error('Failed to get billing status:', statusRes.status, statusRes.statusText);
		throw error(401, 'Failed to get billing status');
	}

	const status = await statusRes.json();
	const customerId: string = status.provider_customer_id ?? '';

	console.log('Portal request details:', {
		customerId,
		server: POLAR_SANDBOX === 'true' ? 'sandbox' : 'production',
		hasAccessToken: !!POLAR_ACCESS_TOKEN,
		accessTokenPrefix: POLAR_ACCESS_TOKEN ? POLAR_ACCESS_TOKEN.substring(0, 10) + '...' : 'none'
	});

	if (!customerId) {
		console.error('No customer ID found in billing status');
		throw error(400, 'No billing account found. Please create a subscription first.');
	}

	const server = POLAR_SANDBOX === 'true' ? 'sandbox' : 'production';
	const returnUrl = `${event.url.origin}/billing`;

	console.log('Creating Polar CustomerPortal handler...');

	const handler = CustomerPortal({
		accessToken: POLAR_ACCESS_TOKEN,
		getCustomerId: async () => {
			console.log('getCustomerId called, returning:', customerId);
			return customerId;
		},
		server,
		returnUrl
	});

	console.log('Calling Polar handler...');
	try {
		const response = await handler(event);
		console.log('Polar handler succeeded');

		// Extract the portal URL from the redirect response
		const portalUrl = response.headers.get('location') || response.url;
		console.log('Portal URL generated:', portalUrl);

		// Return JSON with the portal URL instead of redirecting
		return Response.json({ url: portalUrl });
	} catch (err) {
		console.error('Polar handler error:', err);
		throw err;
	}
};
