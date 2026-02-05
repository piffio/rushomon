import { redirect } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, cookies }) => {
	const token = url.searchParams.get('token');

	if (!token) {
		throw redirect(302, '/?error=missing_token');
	}

	// Set HttpOnly cookie server-side
	// This keeps the session token secure (not accessible via JavaScript)
	cookies.set('rushomon_session', token, {
		path: '/',
		httpOnly: true,
		secure: url.protocol === 'https:',
		sameSite: 'lax',
		maxAge: 604800 // 7 days (matches backend SESSION_TTL_SECONDS)
	});

	throw redirect(302, '/dashboard');
};
