<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { adminApi } from "$lib/api/admin";
	import { authApi } from "$lib/api/auth";
	import type {
		LinkReportWithLink,
		AdminReportsResponse,
		User,
	} from "$lib/types/api";

	let reports = $state<LinkReportWithLink[]>([]);
	let loading = $state(false);
	let error = $state("");
	let currentPage = $state(1);
	let totalPages = $state(0);
	let statusFilter = $state<"all" | "pending" | "reviewed" | "dismissed">(
		"pending",
	);
	let pendingCount = $state(0);
	let currentUser = $state<User | null>(null);
	let authLoading = $state(true);

	// Modal states
	let showBlockUrlModal = $state(false);
	let showBlockDomainModal = $state(false);
	let showDisableModal = $state(false);
	let showDismissModal = $state(false);
	let selectedReport = $state<LinkReportWithLink | null>(null);
	let actionReason = $state("");

	onMount(async () => {
		// First check authentication
		try {
			currentUser = await authApi.me();

			// Check if user is admin
			if (currentUser.role !== "admin") {
				error = "Access denied. Admin privileges required.";
				authLoading = false;
				return;
			}

			// Load reports if authenticated and admin
			await Promise.all([loadReports(), loadPendingCount()]);
		} catch (err: any) {
			console.error("Authentication error:", err);
			if (err.message?.includes("401") || err.status === 401) {
				// Not authenticated, redirect to login
				window.location.href = authApi.getLoginUrl();
				return;
			}
			error = "Failed to authenticate. Please try logging in again.";
		} finally {
			authLoading = false;
		}
	});

	async function loadReports() {
		try {
			loading = true;
			error = "";

			const status = statusFilter === "all" ? undefined : statusFilter;
			const response: AdminReportsResponse = await adminApi.getReports(
				currentPage,
				50,
				status,
			);

			reports = response.reports;
			totalPages = response.pagination.pages;
		} catch (err) {
			console.error("Failed to load reports:", err);
			error = "Failed to load reports. Please try again.";
		} finally {
			loading = false;
		}
	}

	async function loadPendingCount() {
		try {
			const response = await adminApi.getPendingReportsCount();
			pendingCount = response.count;
		} catch (err) {
			console.error("Failed to load pending count:", err);
		}
	}

	async function handlePageChange(page: number) {
		if (page < 1 || page > totalPages) return;
		currentPage = page;
		await loadReports();
	}

	async function handleStatusFilterChange() {
		currentPage = 1;
		await loadReports();
	}

	function getStatusBadge(status: string): string {
		switch (status) {
			case "pending":
				return "warning";
			case "reviewed":
				return "success";
			case "dismissed":
				return "secondary";
			default:
				return "secondary";
		}
	}

	function getStatusLabel(status: string): string {
		switch (status) {
			case "pending":
				return "Pending";
			case "reviewed":
				return "Reviewed";
			case "dismissed":
				return "Dismissed";
			default:
				return status;
		}
	}

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleString();
	}

	function getReporterInfo(report: LinkReportWithLink): string {
		if (report.reporter_user_id) {
			return `User ${report.reporter_user_id.slice(0, 8)}...`;
		}
		if (report.reporter_email) {
			return report.reporter_email;
		}
		return "Anonymous";
	}

	async function handleBlockDestinationURL(report: LinkReportWithLink) {
		selectedReport = report;
		showBlockUrlModal = true;
		actionReason = "";
	}

	async function confirmBlockUrl() {
		if (!selectedReport) return;

		try {
			await adminApi.blockDestination(
				selectedReport.link.destination_url,
				"exact",
				"Reviewed via report",
			);
			// Auto-resolve the report
			await adminApi.updateReportStatus(
				selectedReport.id,
				"reviewed",
				"Action taken: Blocked URL",
			);
			await loadReports();
			await loadPendingCount();
			closeModal();
		} catch (err) {
			console.error("Failed to block destination:", err);
			error = "Failed to block destination. Please try again.";
		}
	}

	async function handleBlockDestinationDomain(report: LinkReportWithLink) {
		selectedReport = report;
		showBlockDomainModal = true;
		actionReason = "";
	}

	async function confirmBlockDomain() {
		if (!selectedReport) return;

		try {
			// Extract domain from URL
			const url = new URL(selectedReport.link.destination_url);
			const domain = url.hostname;

			await adminApi.blockDestination(
				domain,
				"domain",
				"Reviewed via report",
			);
			// Auto-resolve the report
			await adminApi.updateReportStatus(
				selectedReport.id,
				"reviewed",
				"Action taken: Blocked domain",
			);
			await loadReports();
			await loadPendingCount();
			closeModal();
		} catch (err) {
			console.error("Failed to block domain:", err);
			error = "Failed to block domain. Please try again.";
		}
	}

	async function handleDisableLink(report: LinkReportWithLink) {
		selectedReport = report;
		showDisableModal = true;
		actionReason = "";
	}

	async function confirmDisableLink() {
		if (!selectedReport) return;

		try {
			await adminApi.updateLinkStatus(selectedReport.link.id, "disabled");
			// Auto-resolve the report
			await adminApi.updateReportStatus(
				selectedReport.id,
				"reviewed",
				"Action taken: Link disabled",
			);
			await loadReports();
			await loadPendingCount();
			closeModal();
		} catch (err) {
			console.error("Failed to disable link:", err);
			error = "Failed to disable link. Please try again.";
		}
	}

	async function handleDismissReport(report: LinkReportWithLink) {
		selectedReport = report;
		showDismissModal = true;
		actionReason = "";
	}

	async function confirmDismiss() {
		if (!selectedReport) return;

		try {
			await adminApi.updateReportStatus(
				selectedReport.id,
				"dismissed",
				actionReason || "Not a violation",
			);
			await loadReports();
			await loadPendingCount();
			closeModal();
		} catch (err) {
			console.error("Failed to dismiss report:", err);
			error = "Failed to dismiss report. Please try again.";
		}
	}

	function getVerdict(report: LinkReportWithLink): string {
		if (report.status !== "reviewed") return "";

		// Extract verdict from admin_notes
		if (report.admin_notes) {
			const notes = report.admin_notes;
			if (notes.includes("Blocked URL")) return "🚫 Blocked URL";
			if (notes.includes("Blocked domain")) return "🌐 Blocked Domain";
			if (notes.includes("Link disabled")) return "⏸️ Disabled";
			if (notes.includes("Action taken")) return "✅ Action taken";
			return notes; // Fallback to full notes
		}

		// Fallback: check link status
		if (report.link.status === "blocked") return "🚫 Blocked";
		if (report.link.status === "disabled") return "⏸️ Disabled";

		return "✅ Reviewed";
	}

	function closeModal() {
		showBlockUrlModal = false;
		showBlockDomainModal = false;
		showDisableModal = false;
		showDismissModal = false;
		selectedReport = null;
		actionReason = "";
	}
</script>

<svelte:head>
	<title>Reports - Admin Console</title>
</svelte:head>

<div class="reports-page">
	<div class="container mx-auto px-4 py-8">
		<!-- Authentication Loading State -->
		{#if authLoading}
			<div class="text-center py-8">
				<div
					class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"
				></div>
				<p class="mt-2 text-gray-600">Checking authentication...</p>
			</div>
		{:else if error && !currentUser}
			<!-- Authentication Error -->
			<div class="text-center py-8 bg-white rounded-lg shadow-md">
				<div class="mb-4">
					<div class="text-red-600 text-xl mb-2">
						🔒 Authentication Required
					</div>
					<p class="text-gray-700 mb-4">{error}</p>
					<a
						href={authApi.getLoginUrl()}
						class="inline-block px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
					>
						Log in with GitHub
					</a>
				</div>
			</div>
		{:else if currentUser && currentUser.role !== "admin"}
			<!-- Admin Access Denied -->
			<div class="text-center py-8 bg-white rounded-lg shadow-md">
				<div class="mb-4">
					<div class="text-red-600 text-xl mb-2">
						⛔ Access Denied
					</div>
					<p class="text-gray-700 mb-4">
						Admin privileges required to view reports.
					</p>
					<p class="text-gray-600 text-sm">
						Logged in as: {currentUser.email} ({currentUser.role})
					</p>
				</div>
			</div>
		{:else}
			<!-- Main Content (authenticated admin) -->
			<div class="flex justify-between items-center mb-6">
				<h1 class="text-3xl font-bold text-gray-900">Abuse Reports</h1>
				{#if pendingCount > 0}
					<div class="badge badge-warning">
						{pendingCount} pending
					</div>
				{/if}
			</div>

			<!-- Filters -->
			<div class="bg-white rounded-lg shadow-md p-6 mb-6">
				<div class="flex items-center gap-4">
					<label for="status-filter" class="font-medium text-gray-700"
						>Status:</label
					>
					<select
						id="status-filter"
						bind:value={statusFilter}
						onchange={handleStatusFilterChange}
						class="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
					>
						<option value="all">All Reports</option>
						<option value="pending">Pending</option>
						<option value="reviewed">Reviewed</option>
						<option value="dismissed">Dismissed</option>
					</select>
				</div>
			</div>

			<!-- Error Message -->
			{#if error}
				<div
					class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6"
				>
					{error}
				</div>
			{/if}

			<!-- Loading State -->
			{#if loading}
				<div class="text-center py-8">
					<div
						class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"
					></div>
					<p class="mt-2 text-gray-600">Loading reports...</p>
				</div>
			{:else if reports.length === 0}
				<div class="text-center py-8 bg-white rounded-lg shadow-md">
					<p class="text-gray-600">No reports found.</p>
				</div>
			{:else}
				<!-- Reports Table -->
				<div class="bg-white rounded-lg shadow-md overflow-hidden">
					<table class="min-w-full divide-y divide-gray-200">
						<thead class="bg-gray-50">
							<tr>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Report</th
								>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Link</th
								>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Reason</th
								>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Reporter</th
								>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Status</th
								>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Verdict</th
								>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Date</th
								>
								<th
									class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
									>Actions</th
								>
							</tr>
						</thead>
						<tbody class="bg-white divide-y divide-gray-200">
							{#each reports as report, index}
								<tr>
									<td class="px-6 py-4 whitespace-nowrap">
										<div class="text-sm text-gray-900">
											Report #{index + 1}
											{#if report.report_count > 1}
												<span
													class="ml-2 text-xs text-gray-500"
													>({report.report_count} total)</span
												>
											{/if}
										</div>
									</td>
									<td class="px-6 py-4 whitespace-nowrap">
										<div class="text-sm">
											<div
												class="font-medium text-gray-900"
											>
												{report.link.short_code}
											</div>
											<div
												class="text-gray-500 max-w-xs"
												title={report.link
													.destination_url}
											>
												{report.link.destination_url}
											</div>
											<div class="text-gray-400 text-xs">
												by {report.link.creator_email}
											</div>
										</div>
									</td>
									<td class="px-6 py-4">
										<div class="text-sm text-gray-900">
											{report.reason}
										</div>
									</td>
									<td class="px-6 py-4 whitespace-nowrap">
										<div class="text-sm text-gray-900">
											{getReporterInfo(report)}
										</div>
									</td>
									<td class="px-6 py-4 whitespace-nowrap">
										<span
											class="badge badge-{getStatusBadge(
												report.status,
											)}"
											>{getStatusLabel(
												report.status,
											)}</span
										>
									</td>
									<td class="px-6 py-4 whitespace-nowrap">
										{#if report.status === "reviewed"}
											<span
												class="text-sm font-medium"
												title={report.admin_notes}
											>
												{getVerdict(report)}
											</span>
										{:else}
											<span class="text-gray-400 text-sm"
												>-</span
											>
										{/if}
									</td>
									<td
										class="px-6 py-4 whitespace-nowrap text-sm text-gray-500"
									>
										{formatDate(report.created_at)}
									</td>
									<td
										class="px-6 py-4 whitespace-nowrap text-sm font-medium"
									>
										{#if report.status === "pending"}
											<div class="flex flex-wrap gap-2">
												<button
													class="px-3 py-1 bg-blue-600 text-white text-xs rounded hover:bg-blue-700 transition-colors"
													onclick={() =>
														handleBlockDestinationURL(
															report,
														)}
													title="Block this exact URL"
													>Block URL</button
												>
												<button
													class="px-3 py-1 bg-purple-600 text-white text-xs rounded hover:bg-purple-700 transition-colors"
													onclick={() =>
														handleBlockDestinationDomain(
															report,
														)}
													title="Block this entire domain"
													>Block Domain</button
												>
												<button
													class="px-3 py-1 bg-orange-600 text-white text-xs rounded hover:bg-orange-700 transition-colors"
													onclick={() =>
														handleDisableLink(
															report,
														)}
													title="Disable this link only"
													>Disable</button
												>
												<button
													class="px-3 py-1 bg-red-600 text-white text-xs rounded hover:bg-red-700 transition-colors"
													onclick={() =>
														handleDismissReport(
															report,
														)}
													title="Dismiss report"
													>Dismiss</button
												>
											</div>
										{:else}
											<span class="text-gray-400 text-xs"
												>No actions</span
											>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<!-- Pagination -->
				{#if totalPages > 1}
					<div class="flex justify-center items-center gap-2 mt-6">
						<button
							class="px-3 py-1 border border-gray-300 rounded-md disabled:opacity-50"
							disabled={currentPage <= 1}
							onclick={() => handlePageChange(currentPage - 1)}
						>
							Previous
						</button>

						<span class="px-3 py-1"
							>Page {currentPage} of {totalPages}</span
						>

						<button
							class="px-3 py-1 border border-gray-300 rounded-md disabled:opacity-50"
							disabled={currentPage >= totalPages}
							onclick={() => handlePageChange(currentPage + 1)}
						>
							Next
						</button>
					</div>
				{/if}
			{/if}
		{/if}
	</div>
</div>

<!-- Block URL Modal -->
{#if showBlockUrlModal && selectedReport}
	<div
		class="modal-overlay"
		role="button"
		tabindex="0"
		onclick={closeModal}
		onkeydown={(e) => e.key === "Enter" && closeModal()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			onkeydown={(e) => e.key === "Escape" && closeModal()}
		>
			<div class="modal-header">
				<h3 class="modal-title">Block URL</h3>
			</div>
			<div class="modal-body">
				<p>Are you sure you want to block this URL?</p>
				<p>
					<strong>URL:</strong>
					{selectedReport.link.destination_url}
				</p>
				<p>
					<strong>Short code:</strong>
					{selectedReport.link.short_code}
				</p>
			</div>
			<div class="modal-footer">
				<button class="btn-secondary" onclick={closeModal}
					>Cancel</button
				>
				<button class="btn-danger" onclick={confirmBlockUrl}
					>Block URL</button
				>
			</div>
		</div>
	</div>
{/if}

<!-- Block Domain Modal -->
{#if showBlockDomainModal && selectedReport}
	<div
		class="modal-overlay"
		role="button"
		tabindex="0"
		onclick={closeModal}
		onkeydown={(e) => e.key === "Enter" && closeModal()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			onkeydown={(e) => e.key === "Escape" && closeModal()}
		>
			<div class="modal-header">
				<h3 class="modal-title">Block Domain</h3>
			</div>
			<div class="modal-body">
				<p>Are you sure you want to block this entire domain?</p>
				<p>
					<strong>Domain:</strong>
					{new URL(selectedReport.link.destination_url).hostname}
				</p>
				<p>
					<strong>Short code:</strong>
					{selectedReport.link.short_code}
				</p>
			</div>
			<div class="modal-footer">
				<button class="btn-secondary" onclick={closeModal}
					>Cancel</button
				>
				<button class="btn-danger" onclick={confirmBlockDomain}
					>Block Domain</button
				>
			</div>
		</div>
	</div>
{/if}

<!-- Disable Link Modal -->
{#if showDisableModal && selectedReport}
	<div
		class="modal-overlay"
		role="button"
		tabindex="0"
		onclick={closeModal}
		onkeydown={(e) => e.key === "Enter" && closeModal()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			onkeydown={(e) => e.key === "Escape" && closeModal()}
		>
			<div class="modal-header">
				<h3 class="modal-title">Disable Link</h3>
			</div>
			<div class="modal-body">
				<p>Are you sure you want to disable this link?</p>
				<p>
					<strong>URL:</strong>
					{selectedReport.link.destination_url}
				</p>
				<p>
					<strong>Short code:</strong>
					{selectedReport.link.short_code}
				</p>
			</div>
			<div class="modal-footer">
				<button class="btn-secondary" onclick={closeModal}
					>Cancel</button
				>
				<button class="btn-primary" onclick={confirmDisableLink}
					>Disable Link</button
				>
			</div>
		</div>
	</div>
{/if}

<!-- Dismiss Modal -->
{#if showDismissModal && selectedReport}
	<div
		class="modal-overlay"
		role="button"
		tabindex="0"
		onclick={closeModal}
		onkeydown={(e) => e.key === "Enter" && closeModal()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			onkeydown={(e) => e.key === "Escape" && closeModal()}
		>
			<div class="modal-header">
				<h3 class="modal-title">Dismiss Report</h3>
			</div>
			<div class="modal-body">
				<p>Are you sure you want to dismiss this report?</p>
				<p><strong>Reason:</strong> {selectedReport.reason}</p>
				<p>
					<strong>Short code:</strong>
					{selectedReport.link.short_code}
				</p>
				<div class="form-group">
					<label class="form-label" for="dismissal-reason"
						>Reason for dismissal (optional)</label
					>
					<textarea
						id="dismissal-reason"
						class="form-textarea"
						bind:value={actionReason}
						placeholder="Explain why this report is being dismissed..."
					></textarea>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn-secondary" onclick={closeModal}
					>Cancel</button
				>
				<button class="btn-primary" onclick={confirmDismiss}
					>Dismiss Report</button
				>
			</div>
		</div>
	</div>
{/if}

<style>
	.reports-page {
		min-height: 100vh;
		background: #f8fafc;
	}

	.badge {
		padding: 0.25rem 0.5rem;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.badge-warning {
		background: #f59e0b;
		color: white;
	}

	.badge-success {
		background: #10b981;
		color: white;
	}

	.badge-secondary {
		background: #6b7280;
		color: white;
	}

	.container {
		max-width: 1400px;
	}

	.max-w-xs {
		max-width: 12rem;
	}

	/* Modal styles */
	.modal-overlay {
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
		border-radius: 0.5rem;
		padding: 1.5rem;
		max-width: 500px;
		width: 90%;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
	}

	.modal-header {
		margin-bottom: 1rem;
	}

	.modal-title {
		font-size: 1.25rem;
		font-weight: 600;
		color: #111827;
	}

	.modal-body {
		margin-bottom: 1.5rem;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
	}

	.btn-secondary {
		padding: 0.5rem 1rem;
		border: 1px solid #d1d5db;
		background: white;
		color: #374151;
		border-radius: 0.375rem;
		cursor: pointer;
	}

	.btn-secondary:hover {
		background: #f9fafb;
	}

	.btn-danger {
		padding: 0.5rem 1rem;
		background: #dc2626;
		color: white;
		border: none;
		border-radius: 0.375rem;
		cursor: pointer;
	}

	.btn-danger:hover {
		background: #b91c1c;
	}

	.btn-primary {
		padding: 0.5rem 1rem;
		background: #2563eb;
		color: white;
		border: none;
		border-radius: 0.375rem;
		cursor: pointer;
	}

	.btn-primary:hover {
		background: #1d4ed8;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #374151;
	}

	.form-textarea {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #d1d5db;
		border-radius: 0.375rem;
		resize: vertical;
		min-height: 80px;
	}
</style>
