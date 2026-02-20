<script lang="ts">
	import { goto } from "$app/navigation";
	import { apiClient } from "$lib/api/client";
	import type { User } from "$lib/types/api";

	let { data } = $props();
	let currentUser: User | undefined = $derived(data.user);

	let showReportDialog = $state(false);
	let reportingLink = $state("");
	let reason = $state("");
	let reporterEmail = $state("");
	let submitting = $state(false);
	let toast = $state<{
		message: string;
		type: "success" | "error";
		visible: boolean;
	}>({
		message: "",
		type: "success",
		visible: false,
	});

	const reportReasons = [
		{ value: "spam", label: "Spam or misleading content" },
		{ value: "phishing", label: "Phishing or malicious site" },
		{ value: "malware", label: "Malware or virus" },
		{ value: "illegal", label: "Illegal content" },
		{ value: "harassment", label: "Harassment or abuse" },
		{ value: "other", label: "Other" },
	];

	function showToast(message: string, type: "success" | "error") {
		toast.message = message;
		toast.type = type;
		toast.visible = true;
		setTimeout(() => {
			toast.visible = false;
		}, 3000);
	}

	function openReportDialog() {
		const input = document.getElementById("linkId") as HTMLInputElement;
		const value = input.value.trim();
		if (value) {
			reportingLink = value;
			showReportDialog = true;
		}
	}

	async function handleSubmit() {
		if (!reason) return;

		try {
			submitting = true;
			await apiClient.post("/api/reports/links", {
				link_id: reportingLink,
				reason,
				reporter_email:
					currentUser?.email || reporterEmail || undefined,
			});

			showToast(
				"Report submitted successfully. Thank you for helping keep our platform safe.",
				"success",
			);
			setTimeout(() => {
				showReportDialog = false;
				reportingLink = "";
				reason = "";
				reporterEmail = "";
				goto("/?message=report-submitted");
			}, 2000);
		} catch (err) {
			showToast("Failed to submit report. Please try again.", "error");
			console.error(err);
		} finally {
			submitting = false;
		}
	}

	function closeDialog() {
		showReportDialog = false;
		reportingLink = "";
		reason = "";
		reporterEmail = "";
	}
</script>

<svelte:head>
	<title>Report Abuse - Rushomon</title>
	<meta
		name="description"
		content="Report abusive or inappropriate links on Rushomon"
	/>
</svelte:head>

<div class="report-page">
	<div class="container mx-auto px-4 py-8 max-w-4xl">
		<div class="text-center mb-12">
			<h1 class="text-4xl font-bold text-gray-900 mb-4">Report Abuse</h1>
			<p class="text-xl text-gray-600 max-w-2xl mx-auto">
				Help us keep Rushomon safe by reporting links that violate our
				terms of service or contain inappropriate content.
			</p>
		</div>

		<div class="bg-white rounded-lg shadow-md p-8 mb-8">
			<h2 class="text-2xl font-semibold text-gray-900 mb-6">
				How to Report
			</h2>

			<div class="space-y-6">
				<div class="flex items-start gap-4">
					<div
						class="flex-shrink-0 w-8 h-8 bg-orange-100 rounded-full flex items-center justify-center"
					>
						<span class="text-orange-600 font-semibold">1</span>
					</div>
					<div>
						<h3 class="font-semibold text-gray-900 mb-2">
							Find the Link to Report
						</h3>
						<p class="text-gray-600">
							Navigate to the short URL you want to report (e.g.,
							https://r.sh/abc123) or copy the link ID from the
							URL.
						</p>
					</div>
				</div>

				<div class="flex items-start gap-4">
					<div
						class="flex-shrink-0 w-8 h-8 bg-orange-100 rounded-full flex items-center justify-center"
					>
						<span class="text-orange-600 font-semibold">2</span>
					</div>
					<div>
						<h3 class="font-semibold text-gray-900 mb-2">
							Enter Link ID
						</h3>
						<p class="text-gray-600 mb-4">
							Enter the short code or full link ID in the field
							below.
						</p>
						<div class="flex gap-4">
							<input
								type="text"
								id="linkId"
								placeholder="e.g., abc123 or full link ID"
								class="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-orange-500"
							/>
							<button
								onclick={openReportDialog}
								class="px-6 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 transition-colors"
							>
								Report Link
							</button>
						</div>
					</div>
				</div>

				<div class="flex items-start gap-4">
					<div
						class="flex-shrink-0 w-8 h-8 bg-orange-100 rounded-full flex items-center justify-center"
					>
						<span class="text-orange-600 font-semibold">3</span>
					</div>
					<div>
						<h3 class="font-semibold text-gray-900 mb-2">
							Provide Details
						</h3>
						<p class="text-gray-600">
							Explain why you're reporting this link and provide
							any additional context that might help our review
							team.
						</p>
					</div>
				</div>
			</div>
		</div>

		<div class="bg-blue-50 border border-blue-200 rounded-lg p-6 mb-8">
			<h3 class="text-lg font-semibold text-blue-900 mb-3">
				What Happens Next?
			</h3>
			<ul class="space-y-2 text-blue-800">
				<li>
					• Our moderation team will review your report within 24-48
					hours
				</li>
				<li>
					• We'll investigate the reported link and take appropriate
					action
				</li>
				<li>
					• You may be contacted for additional information if needed
				</li>
				<li>• We'll update you on the outcome of our investigation</li>
			</ul>
		</div>

		<div class="bg-gray-50 rounded-lg p-6">
			<h3 class="text-lg font-semibold text-gray-900 mb-3">
				Report Guidelines
			</h3>
			<div class="grid md:grid-cols-2 gap-6">
				<div>
					<h4 class="font-medium text-gray-900 mb-2">Report:</h4>
					<ul class="space-y-1 text-sm text-gray-600">
						<li>• Spam or misleading content</li>
						<li>• Malware, phishing, or security threats</li>
						<li>• Hate speech or harassment</li>
						<li>• Illegal or prohibited content</li>
						<li>• Copyright infringement</li>
					</ul>
				</div>
				<div>
					<h4 class="font-medium text-gray-900 mb-2">
						Do Not Report:
					</h4>
					<ul class="space-y-1 text-sm text-gray-600">
						<li>• Personal disagreements</li>
						<li>• Content you simply disagree with</li>
						<li>• False or malicious reports</li>
						<li>• Duplicate reports for the same issue</li>
					</ul>
				</div>
			</div>
		</div>

		<div class="text-center">
			<p class="text-gray-600 mb-4">
				For questions about our reporting process, please review our
				<a
					href="/terms#abuse-policy"
					class="text-orange-600 hover:text-orange-700"
					>Abuse Policy</a
				>
				or
				<a href="/terms" class="text-orange-600 hover:text-orange-700"
					>Terms of Service</a
				>.
			</p>
			<a
				href="/"
				class="inline-flex items-center gap-2 text-orange-600 hover:text-orange-700 transition-colors"
			>
				← Back to Home
			</a>
		</div>
	</div>
</div>

<!-- Report Dialog -->
{#if showReportDialog}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={closeDialog}
		onkeydown={(e) => e.key === "Enter" && closeDialog()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
		>
			<div class="modal-header">
				<h3>Report Link: {reportingLink}</h3>
				<button class="modal-close" onclick={closeDialog}
					>&times;</button
				>
			</div>
			<div class="modal-body">
				<div class="form-group">
					<label>Reason for Report</label>
					<select bind:value={reason} class="form-select">
						<option value="">Select a reason</option>
						{#each reportReasons as reasonOption}
							<option value={reasonOption.value}
								>{reasonOption.label}</option
							>
						{/each}
					</select>
				</div>
				<div class="form-group">
					<label
						>Your Email {currentUser
							? "(authenticated)"
							: "(optional)"}</label
					>
					{#if currentUser}
						<div class="authenticated-user-info">
							<span class="user-email">{currentUser.email}</span>
							<span class="user-badge">Logged in</span>
						</div>
					{:else}
						<input
							type="email"
							bind:value={reporterEmail}
							placeholder="your@email.com"
							class="form-input"
						/>
					{/if}
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" onclick={closeDialog}>
					Cancel
				</button>
				<button
					class="btn btn-danger"
					onclick={handleSubmit}
					disabled={!reason || submitting}
				>
					{submitting ? "Submitting..." : "Submit Report"}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Toast Notification -->
{#if toast.visible}
	<div class="toast {toast.type}" class:visible={toast.visible}>
		{toast.message}
	</div>
{/if}

<style>
	.report-page {
		min-height: 100vh;
		background: linear-gradient(135deg, #f8fafc 0%, #e2e8f0 100%);
	}

	.container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.toast {
		position: fixed;
		top: 1rem;
		right: 1rem;
		padding: 1rem 1.5rem;
		border-radius: 6px;
		color: white;
		font-weight: 500;
		z-index: 1001;
		opacity: 0;
		transform: translateY(-10px);
		transition: all 0.2s;
	}

	.toast.visible {
		opacity: 1;
		transform: translateY(0);
	}

	.toast.success {
		background: #059669;
	}

	.toast.error {
		background: #dc2626;
	}

	/* Modal Styles */
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 8px;
		width: 90%;
		max-width: 500px;
		max-height: 90vh;
		overflow-y: auto;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e2e8f0;
	}

	.modal-header h3 {
		margin: 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #1e293b;
	}

	.modal-close {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: #64748b;
		padding: 0;
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e2e8f0;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #374151;
	}

	.form-input,
	.form-select {
		width: 100%;
		padding: 0.5rem 1rem;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.form-select {
		cursor: pointer;
	}

	.btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-secondary {
		background: #64748b;
		color: white;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #475569;
	}

	.btn-danger {
		background: #dc2626;
		color: white;
	}

	.btn-danger:hover:not(:disabled) {
		background: #b91c1c;
	}

	.authenticated-user-info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		background: #f0fdf4;
		border: 1px solid #bbf7d0;
		border-radius: 6px;
	}

	.user-email {
		font-weight: 500;
		color: #166534;
	}

	.user-badge {
		font-size: 0.75rem;
		font-weight: 500;
		padding: 0.25rem 0.5rem;
		background: #22c55e;
		color: white;
		border-radius: 9999px;
	}
</style>
