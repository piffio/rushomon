<script lang="ts">
	import { orgsApi } from "$lib/api/orgs";
	import type { PageData } from "./$types";
	import type { InviteInfo } from "$lib/types/api";

	let { data }: { data: PageData } = $props();

	let inviteInfo = $state<InviteInfo | null>(null);
	let loading = $state(true);
	let accepting = $state(false);
	let accepted = $state(false);
	let error = $state("");

	$effect(() => {
		if (accepted) return; // Don't fetch if already accepted
		orgsApi
			.getInviteInfo(data.token)
			.then((info) => {
				inviteInfo = info;
			})
			.catch(() => {
				error = "Failed to load invitation details.";
			})
			.finally(() => {
				loading = false;
			});
	});

	async function handleAccept() {
		if (!inviteInfo?.valid) return;
		accepting = true;
		error = "";
		try {
			await orgsApi.acceptInvite(data.token);
			accepted = true;
			// Clear the stored token to force refresh from the new cookie set by backend
			if (typeof localStorage !== "undefined") {
				localStorage.removeItem("rushomon_access_token");
				localStorage.removeItem("pending_invite_token");
			}
			// Redirect to dashboard instead of reloading the page
			setTimeout(() => {
				window.location.href = "/dashboard";
			}, 1500);
		} catch (e: any) {
			error = e?.message ?? "Failed to accept invitation.";
		} finally {
			accepting = false;
		}
	}

	function formatDate(ts: number): string {
		return new Date(ts * 1000).toLocaleDateString(undefined, {
			year: "numeric",
			month: "long",
			day: "numeric",
		});
	}
</script>

<div class="min-h-screen bg-gray-50 flex items-center justify-center px-4">
	<div class="w-full max-w-md">
		<!-- Logo / Brand -->
		<div class="text-center mb-8">
			<a href="/" class="inline-flex items-center gap-2">
				<span class="text-2xl font-bold text-gray-900">rushomon</span>
				<span class="text-orange-500 text-2xl font-bold">.</span>
			</a>
		</div>

		<div class="bg-white rounded-2xl border border-gray-200 shadow-sm p-8">
			{#if loading}
				<div class="flex flex-col items-center py-8 gap-4">
					<div
						class="w-10 h-10 border-2 border-orange-500 border-t-transparent rounded-full animate-spin"
					></div>
					<p class="text-gray-500 text-sm">Loading invitation…</p>
				</div>
			{:else if accepted}
				<div class="flex flex-col items-center py-4 gap-4 text-center">
					<div
						class="w-16 h-16 rounded-full bg-green-100 flex items-center justify-center"
					>
						<svg
							class="w-8 h-8 text-green-600"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M5 13l4 4L19 7"
							/>
						</svg>
					</div>
					<h1 class="text-xl font-bold text-gray-900">You're in!</h1>
					<p class="text-gray-500 text-sm">
						Redirecting you to the dashboard…
					</p>
				</div>
			{:else if !inviteInfo?.valid}
				<div class="flex flex-col items-center py-4 gap-4 text-center">
					<div
						class="w-16 h-16 rounded-full bg-red-100 flex items-center justify-center"
					>
						<svg
							class="w-8 h-8 text-red-500"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
					</div>
					<h1 class="text-xl font-bold text-gray-900">
						Invitation not valid
					</h1>
					<p class="text-gray-500 text-sm">
						{#if inviteInfo?.reason === "already_accepted"}
							This invitation has already been accepted.
						{:else if inviteInfo?.reason === "expired"}
							This invitation has expired.
						{:else}
							This invitation link is invalid or has been revoked.
						{/if}
					</p>
					<a
						href="/dashboard"
						class="mt-2 text-sm font-medium text-orange-600 hover:text-orange-700 transition-colors"
					>
						Go to Dashboard →
					</a>
				</div>
			{:else}
				<!-- Valid invitation -->
				<div class="text-center mb-6">
					<div
						class="w-16 h-16 rounded-xl bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-2xl font-bold mx-auto mb-4"
					>
						{inviteInfo.org_name?.charAt(0).toUpperCase() ?? "O"}
					</div>
					<h1 class="text-xl font-bold text-gray-900">
						You've been invited!
					</h1>
					<p class="text-gray-600 mt-2">
						<span class="font-medium">{inviteInfo.invited_by}</span>
						invited you to join
						<span class="font-medium text-gray-900"
							>{inviteInfo.org_name}</span
						>
					</p>
					{#if inviteInfo.expires_at}
						<p class="text-xs text-gray-400 mt-1">
							Expires {formatDate(inviteInfo.expires_at)}
						</p>
					{/if}
				</div>

				{#if error}
					<div
						class="mb-4 bg-red-50 border border-red-200 rounded-lg p-3 text-sm text-red-700"
					>
						{error}
					</div>
				{/if}

				{#if data.user}
					<!-- Already logged in: show accept button -->
					<div
						class="bg-gray-50 rounded-xl p-4 mb-5 text-sm text-gray-700"
					>
						You're signed in as <span class="font-medium"
							>{data.user.email}</span
						>.
						{#if inviteInfo.email && data.user.email.toLowerCase() !== inviteInfo.email.toLowerCase()}
							<p class="mt-1 text-amber-700 font-medium">
								⚠️ This invite was sent to <span
									class="font-semibold"
									>{inviteInfo.email}</span
								>. Please sign in with that account to accept.
							</p>
						{/if}
					</div>

					<button
						onclick={handleAccept}
						disabled={accepting ||
							(!!inviteInfo.email &&
								data.user.email.toLowerCase() !==
									inviteInfo.email.toLowerCase())}
						class="w-full py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-xl font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{accepting ? "Accepting…" : "Accept Invitation"}
					</button>
				{:else}
					<!-- Not logged in: prompt to sign in -->
					<p class="text-sm text-gray-600 mb-5 text-center">
						Sign in to accept this invitation. Your account email
						must match
						<span class="font-medium">{inviteInfo.email}</span>.
					</p>
					<a
						href="/login?redirect=/invite/{data.token}"
						class="block w-full py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-xl font-semibold text-center hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm"
					>
						Sign In to Accept
					</a>
				{/if}
			{/if}
		</div>
	</div>
</div>
