import { Webhooks } from '@polar-sh/sveltekit';
import type { RequestHandler } from './$types';
import {
	POLAR_WEBHOOK_SECRET,
	INTERNAL_WEBHOOK_SECRET
} from '$env/static/private';
import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';

async function notifyWorker(eventType: string, payload: Record<string, unknown>): Promise<void> {
	const workerBase = PUBLIC_VITE_API_BASE_URL;
	await fetch(`${workerBase}/api/billing/subscription-update`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			'X-Internal-Secret': INTERNAL_WEBHOOK_SECRET
		},
		body: JSON.stringify({ event_type: eventType, ...payload })
	});
}

function toUnix(d: Date | string | null | undefined): number {
	if (!d) return 0;
	return Math.floor(new Date(d).getTime() / 1000);
}

export const POST: RequestHandler = async (event) => {
	try {
		const handler = Webhooks({
			webhookSecret: POLAR_WEBHOOK_SECRET,

			onSubscriptionActive: async (payload) => {
				const sub = payload.data;
				console.log('Webhook: subscription_active received', {
					customerId: sub.customerId,
					externalId: sub.customer?.externalId,
					subscriptionId: sub.id
				});
				await notifyWorker('subscription_activated', {
					subscription_id: sub.id,
					customer_id: sub.customerId,
					billing_account_id: sub.customer?.externalId ?? '',
					price_id: sub.prices?.[0]?.id ?? '',
					interval: sub.recurringInterval ?? 'month'
				});
			},

			onSubscriptionCreated: async (payload) => {
				const sub = payload.data;
				console.log('Webhook: subscription_created received', {
					customerId: sub.customerId,
					externalId: sub.customer?.externalId,
					subscriptionId: sub.id
				});
				await notifyWorker('subscription_created', {
					subscription_id: sub.id,
					customer_id: sub.customerId,
					billing_account_id: sub.customer?.externalId ?? '',
					price_id: sub.prices?.[0]?.id ?? '',
					interval: sub.recurringInterval ?? 'month'
				});
			},

			onSubscriptionUpdated: async (payload) => {
				const sub = payload.data;
				await notifyWorker('subscription_updated', {
					subscription_id: sub.id,
					customer_id: sub.customerId,
					billing_account_id: sub.customer?.externalId ?? '',
					status: sub.status,
					price_id: sub.prices?.[0]?.id ?? '',
					interval: sub.recurringInterval ?? 'month',
					current_period_start: toUnix(sub.currentPeriodStart),
					current_period_end: toUnix(sub.currentPeriodEnd),
					cancel_at_period_end: sub.cancelAtPeriodEnd ?? false
				});
			},

			onSubscriptionCanceled: async (payload) => {
				const sub = payload.data;
				await notifyWorker('subscription_canceled', {
					subscription_id: sub.id,
					customer_id: sub.customerId,
					billing_account_id: sub.customer?.externalId ?? ''
				});
			},

			onSubscriptionRevoked: async (payload) => {
				const sub = payload.data;
				await notifyWorker('subscription_revoked', {
					subscription_id: sub.id,
					customer_id: sub.customerId,
					billing_account_id: sub.customer?.externalId ?? ''
				});
			},

			onSubscriptionUncanceled: async (payload) => {
				const sub = payload.data;
				await notifyWorker('subscription_updated', {
					subscription_id: sub.id,
					customer_id: sub.customerId,
					status: sub.status,
					price_id: sub.prices?.[0]?.id ?? '',
					interval: sub.recurringInterval ?? 'month',
					current_period_start: toUnix(sub.currentPeriodStart),
					current_period_end: toUnix(sub.currentPeriodEnd),
					cancel_at_period_end: false
				});
			}
		}) as unknown as RequestHandler;

		return await handler(event);
	} catch (error) {
		console.error('Webhook error:', error);
		// Return 200 to prevent Polar from retrying
		return new Response('OK', { status: 200 });
	}
};
