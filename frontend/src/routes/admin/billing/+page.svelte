<script lang="ts">
	import { onMount } from "svelte";
	import { adminApi } from "$lib/api/admin";
	import type {
		BillingAccountWithStats,
		BillingAccountDetails,
	} from "$lib/types/api";

	let accounts = $state<BillingAccountWithStats[]>([]);
	let total = $state(0);
	let loading = $state(false);
	let error = $state("");
	let currentPage = $state(1);
	let searchQuery = $state("");
	let tierFilter = $state("");
	let expandedId = $state<string | null>(null);
	let accountDetails = $state<Record<string, BillingAccountDetails>>({});
	let detailsLoading = $state<string | null>(null);
	let confirmingTierChange = $state<{
		accountId: string;
		currentTier: string;
	} | null>(null);
	let tierLoading = $state(false);
	let confirmingReset = $state<string | null>(null);

	onMount(async () => {
		await loadAccounts();

		// Check if there's a hash in the URL to expand a specific account
		const hash = window.location.hash.slice(1); // Remove the # character
		if (hash && hash.startsWith('ba_')) {
			expandedId = hash;
			// Load details for this account
			if (!accountDetails[hash]) {
				try {
					detailsLoading = hash;
					const details = await adminApi.getBillingAccount(hash);
					accountDetails[hash] = details;
				} catch (err) {
					console.error('Failed to load account from hash:', err);
				} finally {
					detailsLoading = null;
				}
			}
			// Scroll to the account
			setTimeout(() => {
				const element = document.getElementById(`account-${hash}`);
				if (element) {
					element.scrollIntoView({ behavior: 'smooth', block: 'start' });
				}
			}, 100);
		}
	});

	async function loadAccounts() {
		try {
			loading = true;
			const response = await adminApi.listBillingAccounts(
				currentPage,
				50,
				searchQuery || undefined,
				tierFilter || undefined,
			);
			accounts = response.accounts;
			total = response.total;
		} catch (err) {
			error = "Failed to load billing accounts";
			console.error(err);
		} finally {
			loading = false;
		}
	}

	async function toggleExpand(accountId: string) {
		if (expandedId === accountId) {
			expandedId = null;
		} else {
			expandedId = accountId;
			// Load details if not already loaded
			if (!accountDetails[accountId]) {
				try {
					detailsLoading = accountId;
					const details = await adminApi.getBillingAccount(accountId);
					accountDetails[accountId] = details;
				} catch (err) {
					error = "Failed to load account details";
					console.error(err);
				} finally {
					detailsLoading = null;
				}
			}
		}
	}

	async function handleTierChange(accountId: string, currentTier: string) {
		confirmingTierChange = { accountId, currentTier };
	}

	async function confirmTierChange(newTier: string) {
		if (!confirmingTierChange) return;

		try {
			tierLoading = true;
			await adminApi.updateBillingAccountTier(
				confirmingTierChange.accountId,
				newTier,
			);
			// Reload accounts to reflect changes
			await loadAccounts();
			// Reload details if expanded
			if (expandedId === confirmingTierChange.accountId) {
				const details = await adminApi.getBillingAccount(
					confirmingTierChange.accountId,
				);
				accountDetails[confirmingTierChange.accountId] = details;
			}
		} catch (err) {
			error = "Failed to update tier";
			console.error(err);
		} finally {
			tierLoading = false;
			confirmingTierChange = null;
		}
	}

	function cancelTierChange() {
		confirmingTierChange = null;
	}

	async function handleResetCounter(accountId: string) {
		confirmingReset = accountId;
	}

	async function confirmReset() {
		if (!confirmingReset) return;

		try {
			loading = true;
			await adminApi.resetBillingAccountCounter(confirmingReset);
			// Reload details to show updated usage
			if (expandedId === confirmingReset) {
				const details = await adminApi.getBillingAccount(confirmingReset);
				accountDetails[confirmingReset] = details;
			}
			await loadAccounts();
		} catch (err) {
			error = "Failed to reset counter";
			console.error(err);
		} finally {
			loading = false;
			confirmingReset = null;
		}
	}

	function cancelReset() {
		confirmingReset = null;
	}

	async function handlePageChange(page: number) {
		if (page < 1) return;
		currentPage = page;
		await loadAccounts();
	}

	function formatDate(ts: number | string): string {
		const ms = typeof ts === "number" ? ts * 1000 : Number(ts) * 1000;
		return new Date(ms).toLocaleDateString();
	}

	function getTierBadgeClass(tier: string): string {
		switch (tier.toLowerCase()) {
			case "free":
				return "tier-free";
			case "pro":
				return "tier-pro";
			case "business":
				return "tier-business";
			case "unlimited":
				return "tier-unlimited";
			default:
				return "tier-free";
		}
	}

	function getTierDisplayName(tier: string): string {
		return tier.charAt(0).toUpperCase() + tier.slice(1);
	}

	const totalPages = $derived(Math.ceil(total / 50));
</script>

<div class="billing-page">
	<div class="page-header">
		<h1>Billing Accounts</h1>
		<p class="subtitle">Manage subscription tiers and quotas</p>
	</div>

	<!-- Filters -->
	<div class="filters">
		<input
			type="text"
			bind:value={searchQuery}
			placeholder="Search by email..."
			class="search-input"
			onchange={() => loadAccounts()}
		/>
		<select
			bind:value={tierFilter}
			class="tier-filter"
			onchange={() => loadAccounts()}
		>
			<option value="">All tiers</option>
			<option value="free">Free</option>
			<option value="pro">Pro</option>
			<option value="business">Business</option>
			<option value="unlimited">Unlimited</option>
		</select>
	</div>

	{#if loading && accounts.length === 0}
		<div class="loading">Loading billing accounts...</div>
	{:else if error && accounts.length === 0}
		<div class="error">{error}</div>
	{:else}
		<div class="accounts-list">
			{#each accounts as account (account.id)}
				<div
					id="account-{account.id}"
					class="account-card {expandedId === account.id
						? 'expanded'
						: ''}"
				>
					<!-- Collapsed Header -->
					<button
						class="card-header"
						onclick={() => toggleExpand(account.id)}
					>
						<span class="owner">{account.owner_email}</span>
						<span
							class="tier-badge {getTierBadgeClass(account.tier)}"
							>{getTierDisplayName(account.tier)}</span
						>
						<span class="usage"
							>{account.links_created_this_month} links</span
						>
						<span class="orgs">{account.org_count} org(s)</span>
						<span class="icon"
							>{expandedId === account.id ? "▼" : "▶"}</span
						>
					</button>

					<!-- Expanded Body -->
					{#if expandedId === account.id}
						<div class="card-body">
							{#if detailsLoading === account.id}
								<div class="loading-details">
									Loading details...
								</div>
							{:else if accountDetails[account.id]}
								{@const details = accountDetails[account.id]}

								<!-- Tier Management -->
								<div class="section">
									<h4>Account Information</h4>
									<div class="info-grid">
										<div class="info-item">
											<span class="label">Owner:</span>
											<span class="value"
												>{details.owner.email} ({details
													.owner.name ||
													"No name"})</span
											>
										</div>
										<div class="info-item">
											<span class="label">Tier:</span>
											<button
												class="tier-change-btn {getTierBadgeClass(
													details.account.tier,
												)}"
												onclick={() =>
													handleTierChange(
														account.id,
														details.account.tier,
													)}
												disabled={tierLoading}
											>
												{getTierDisplayName(
													details.account.tier,
												)} ▼
											</button>
										</div>
										<div class="info-item">
											<span class="label">Created:</span>
											<span class="value"
												>{formatDate(
													details.account.created_at,
												)}</span
											>
										</div>
									</div>
								</div>

								<!-- Usage Stats -->
								<div class="section">
									<h4>
										Usage This Month ({details.usage
											.year_month})
									</h4>
									<div class="usage-bar">
										<div class="usage-text">
											{details.usage
												.links_created_this_month} /
											{details.usage
												.max_links_per_month ??
												"∞"} links
										</div>
										{#if details.usage.max_links_per_month}
											{@const percentage =
												(details.usage
													.links_created_this_month /
													details.usage
														.max_links_per_month) *
												100}
											<div class="progress-bar">
												<div
													class="progress-fill"
													style="width: {Math.min(
														percentage,
														100,
													)}%"
												></div>
											</div>
										{/if}
									</div>
								</div>

								<!-- Organizations -->
								<div class="section">
									<h4>
										Organizations ({details.organizations
											.length})
									</h4>
									{#if details.organizations.length === 0}
										<p class="no-orgs">
											No organizations yet
										</p>
									{:else}
										<div class="orgs-list">
											{#each details.organizations as org}
												<div class="org-card">
													<div class="org-header">
														<h5>{org.name}</h5>
														<span class="org-slug"
															>/{org.slug}</span
														>
													</div>
													<div class="org-stats">
														<span
															>{org.link_count} links</span
														>
														<span
															>{org.member_count} members</span
														>
														<span
															>Created {formatDate(
																org.created_at,
															)}</span
														>
													</div>
												</div>
											{/each}
										</div>
									{/if}
								</div>

								<!-- Admin Actions -->
								<div class="section actions-section">
									<h4>Admin Actions</h4>
									<button
										class="btn-danger"
										onclick={() =>
											handleResetCounter(account.id)}
										disabled={loading}
									>
										Reset Counter (Current Month)
									</button>
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Pagination -->
		{#if totalPages > 1}
			<div class="pagination">
				<p class="pagination-info">
					Page {currentPage} of {totalPages} ({total} total accounts)
				</p>
				<div class="pagination-controls">
					<button
						onclick={() => handlePageChange(currentPage - 1)}
						disabled={currentPage <= 1 || loading}
						class="pagination-btn"
					>
						Previous
					</button>
					<button
						onclick={() => handlePageChange(currentPage + 1)}
						disabled={currentPage >= totalPages || loading}
						class="pagination-btn"
					>
						Next
					</button>
				</div>
			</div>
		{/if}
	{/if}
</div>

<!-- Tier Change Modal -->
{#if confirmingTierChange}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={cancelTierChange}
		onkeydown={(e) => e.key === "Enter" && cancelTierChange()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
		>
			<div class="modal-header">
				<h3>Change Billing Account Tier</h3>
				<button class="modal-close" onclick={cancelTierChange}
					>&times;</button
				>
			</div>
			<div class="modal-body">
				<p>
					Select new tier for this billing account. This will affect
					all organizations linked to this account.
				</p>
				<div class="tier-options">
					{#each ["free", "pro", "business", "unlimited"] as tier}
						<button
							class="tier-option {tier === confirmingTierChange.currentTier
								? 'current'
								: ''} {getTierBadgeClass(tier)}"
							onclick={() => confirmTierChange(tier)}
							disabled={tierLoading ||
								tier === confirmingTierChange.currentTier}
						>
							{getTierDisplayName(tier)}
							{#if tier === confirmingTierChange.currentTier}
								<span class="current-label">(current)</span>
							{/if}
						</button>
					{/each}
				</div>
			</div>
			<div class="modal-footer">
				<button
					class="btn btn-secondary"
					onclick={cancelTierChange}
					disabled={tierLoading}
				>
					Cancel
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Reset Counter Modal -->
{#if confirmingReset}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={cancelReset}
		onkeydown={(e) => e.key === "Enter" && cancelReset()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
		>
			<div class="modal-header">
				<h3>Reset Counter?</h3>
				<button class="modal-close" onclick={cancelReset}
					>&times;</button
				>
			</div>
			<div class="modal-body">
				<p>
					Are you sure you want to <strong
						>reset the monthly counter</strong
					> for this billing account? This will set the links created
					count to 0 for the current month.
				</p>
				<p class="warning">
					This action is typically used for testing purposes only.
				</p>
			</div>
			<div class="modal-footer">
				<button
					class="btn btn-secondary"
					onclick={cancelReset}
					disabled={loading}
				>
					Cancel
				</button>
				<button
					class="btn btn-danger"
					onclick={confirmReset}
					disabled={loading}
				>
					{#if loading}
						Resetting...
					{:else}
						Reset Counter
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if error && accounts.length > 0}
	<div class="toast-error">{error}</div>
{/if}

<style>
	.billing-page {
		max-width: 1200px;
		margin: 0 auto;
	}

	.page-header {
		margin-bottom: 2rem;
	}

	.page-header h1 {
		margin: 0 0 0.5rem 0;
		font-size: 1.75rem;
		font-weight: 600;
		color: #1e293b;
	}

	.subtitle {
		margin: 0;
		color: #64748b;
		font-size: 1rem;
	}

	.filters {
		display: flex;
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.search-input,
	.tier-filter {
		padding: 0.5rem 1rem;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.search-input {
		flex: 1;
		max-width: 400px;
	}

	.tier-filter {
		width: 150px;
	}

	.loading,
	.error {
		text-align: center;
		padding: 3rem;
		color: #64748b;
	}

	.error {
		color: #dc2626;
	}

	.accounts-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.account-card {
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 8px;
		overflow: hidden;
		transition: box-shadow 0.2s;
	}

	.account-card.expanded {
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
	}

	.card-header {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem 1.5rem;
		background: white;
		border: none;
		cursor: pointer;
		transition: background 0.2s;
		text-align: left;
	}

	.card-header:hover {
		background: #f8fafc;
	}

	.owner {
		flex: 1;
		font-weight: 500;
		color: #1e293b;
	}

	.tier-badge {
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
		text-transform: uppercase;
	}

	.tier-free {
		background: #dbeafe;
		color: #1e40af;
	}

	.tier-pro {
		background: #fef3c7;
		color: #92400e;
	}

	.tier-business {
		background: #d1fae5;
		color: #065f46;
	}

	.tier-unlimited {
		background: #e0e7ff;
		color: #3730a3;
	}

	.usage,
	.orgs {
		color: #64748b;
		font-size: 0.875rem;
	}

	.icon {
		color: #9ca3af;
		font-size: 0.75rem;
	}

	.card-body {
		padding: 1.5rem;
		background: #f8fafc;
		border-top: 1px solid #e2e8f0;
	}

	.loading-details {
		text-align: center;
		padding: 2rem;
		color: #64748b;
	}

	.section {
		margin-bottom: 2rem;
	}

	.section:last-child {
		margin-bottom: 0;
	}

	.section h4 {
		margin: 0 0 1rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #1e293b;
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1rem;
	}

	.info-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.info-item .label {
		font-weight: 500;
		color: #64748b;
	}

	.info-item .value {
		color: #1e293b;
	}

	.tier-change-btn {
		padding: 0.25rem 0.75rem;
		border: none;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		text-transform: uppercase;
	}

	.tier-change-btn:hover:not(:disabled) {
		transform: scale(1.05);
	}

	.tier-change-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.usage-bar {
		background: white;
		padding: 1rem;
		border-radius: 6px;
	}

	.usage-text {
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #1e293b;
	}

	.progress-bar {
		height: 8px;
		background: #e2e8f0;
		border-radius: 4px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: linear-gradient(to right, #3b82f6, #2563eb);
		transition: width 0.3s;
	}

	.no-orgs {
		color: #64748b;
		font-style: italic;
	}

	.orgs-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.org-card {
		background: white;
		padding: 1rem;
		border-radius: 6px;
		border: 1px solid #e2e8f0;
	}

	.org-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.org-header h5 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		color: #1e293b;
	}

	.org-slug {
		color: #64748b;
		font-size: 0.875rem;
	}

	.org-stats {
		display: flex;
		gap: 1rem;
		color: #64748b;
		font-size: 0.875rem;
	}

	.actions-section button {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-danger {
		background: #dc2626;
		color: white;
	}

	.btn-danger:hover:not(:disabled) {
		background: #b91c1c;
	}

	.btn-danger:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.pagination {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		margin-top: 2rem;
		background: white;
		border-radius: 8px;
		border: 1px solid #e2e8f0;
	}

	.pagination-info {
		color: #64748b;
		font-size: 0.875rem;
		margin: 0;
	}

	.pagination-controls {
		display: flex;
		gap: 0.5rem;
	}

	.pagination-btn {
		padding: 0.5rem 1rem;
		border: 1px solid #d1d5db;
		background: white;
		color: #374151;
		border-radius: 6px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.pagination-btn:hover:not(:disabled) {
		background: #f9fafb;
		border-color: #9ca3af;
	}

	.pagination-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
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

	.modal-body p {
		margin: 0 0 1rem 0;
		color: #475569;
		line-height: 1.5;
	}

	.modal-body p:last-of-type {
		margin-bottom: 0;
	}

	.warning {
		color: #dc2626;
		font-weight: 500;
	}

	.tier-options {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.tier-option {
		padding: 1rem;
		border: 2px solid transparent;
		border-radius: 8px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		text-transform: uppercase;
	}

	.tier-option:hover:not(:disabled) {
		transform: scale(1.05);
		border-color: #3b82f6;
	}

	.tier-option.current {
		border-color: #94a3b8;
		opacity: 0.6;
	}

	.tier-option:disabled {
		cursor: not-allowed;
	}

	.current-label {
		display: block;
		font-size: 0.75rem;
		text-transform: none;
		margin-top: 0.25rem;
		opacity: 0.7;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e2e8f0;
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

	.btn-secondary {
		background: #64748b;
		color: white;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #475569;
	}

	.btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.toast-error {
		position: fixed;
		bottom: 2rem;
		right: 2rem;
		background: #dc2626;
		color: white;
		padding: 1rem 1.5rem;
		border-radius: 6px;
		box-shadow: 0 10px 15px rgba(0, 0, 0, 0.2);
		z-index: 1000;
	}
</style>
