<script lang="ts">
	import { onMount } from "svelte";
	import { adminApi } from "$lib/api/admin";

	let settings = $state<Record<string, string>>({});
	let loading = $state(false);
	let saving = $state(false);
	let showToast = $state(false);
	let toastMessage = $state("");
	let signupsEnabled = $state(true);
	let defaultUserTier = $state("free");
	let confirmingSignupToggle = $state(false);

	onMount(() => {
		loadSettings();
	});

	async function loadSettings() {
		try {
			loading = true;
			settings = await adminApi.getSettings();
			signupsEnabled = settings.signups_enabled !== "false";
			defaultUserTier = settings.default_user_tier || "free";
		} catch (err) {
			console.error("Failed to load settings:", err);
		} finally {
			loading = false;
		}
	}

	async function handleSignupToggle() {
		confirmingSignupToggle = true;
	}

	async function confirmSignupToggle() {
		saving = true;
		try {
			const newValue = signupsEnabled ? "false" : "true";
			const updatedSettings = await adminApi.updateSetting(
				"signups_enabled",
				newValue,
			);
			settings = updatedSettings;
			signupsEnabled = updatedSettings.signups_enabled !== "false";
			showToastMessage(
				`Signups ${signupsEnabled ? "enabled" : "disabled"}`,
			);
		} catch (err) {
			console.error("Failed to update setting:", err);
			showToastMessage("Failed to update setting");
		} finally {
			saving = false;
			confirmingSignupToggle = false;
		}
	}

	async function handleDefaultTierChange() {
		saving = true;
		try {
			const updatedSettings = await adminApi.updateSetting(
				"default_user_tier",
				defaultUserTier,
			);
			settings = updatedSettings;
			defaultUserTier = updatedSettings.default_user_tier || "free";
			showToastMessage(`Default tier updated to ${defaultUserTier}`);
		} catch (err) {
			console.error("Failed to update setting:", err);
			showToastMessage("Failed to update setting");
		} finally {
			saving = false;
		}
	}

	async function handleUpdateSetting(key: string, value: string) {
		try {
			saving = true;
			settings = await adminApi.updateSetting(key, value);
			showToastMessage("Setting updated");
		} catch (err) {
			console.error("Failed to update setting:", err);
			showToastMessage("Failed to update setting");
		} finally {
			saving = false;
		}
	}

	function showToastMessage(message: string) {
		toastMessage = message;
		showToast = true;
		setTimeout(() => {
			showToast = false;
		}, 3000);
	}
</script>

<div class="settings-page">
	<div class="page-header">
		<h1>Instance Settings</h1>
		<p class="subtitle">Configure system-wide settings and policies</p>
	</div>

	{#if loading}
		<div class="loading">Loading settings...</div>
	{:else}
		<div class="settings-container">
			<!-- Registration Setting -->
			<div class="setting-card">
				<div class="setting-content">
					<div class="setting-info">
						<h3>Allow new signups</h3>
						<p class="setting-description">
							Control whether new users can create accounts on
							this instance
						</p>
					</div>
					<div class="setting-control">
						<button
							onclick={handleSignupToggle}
							disabled={saving}
							class="toggle-switch {signupsEnabled
								? 'enabled'
								: 'disabled'}"
							role="switch"
							aria-checked={signupsEnabled}
							aria-label="Toggle new signups"
						>
							<span class="toggle-slider"></span>
						</button>
					</div>
				</div>
			</div>

			<!-- Default User Tier Setting -->
			<div class="setting-card">
				<div class="setting-content">
					<div class="setting-info">
						<h3>Default tier for new users</h3>
						<p class="setting-description">
							New signups will be assigned this tier by default
						</p>
					</div>
					<div class="setting-control">
						<select
							bind:value={defaultUserTier}
							onchange={handleDefaultTierChange}
							disabled={saving}
							class="tier-select"
						>
							<option value="free">Free</option>
							<option value="unlimited">Unlimited</option>
						</select>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<!-- Signup Toggle Confirmation Modal -->
{#if confirmingSignupToggle}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={() => (confirmingSignupToggle = false)}
		onkeydown={(e) => e.key === "Enter" && (confirmingSignupToggle = false)}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			onkeydown={(e) =>
				e.key === "Escape" && (confirmingSignupToggle = false)}
		>
			<div class="modal-header">
				<h3>
					{signupsEnabled
						? "Disable New Signups?"
						: "Enable New Signups?"}
				</h3>
				<button
					class="modal-close"
					onclick={() => (confirmingSignupToggle = false)}
					>&times;</button
				>
			</div>
			<div class="modal-body">
				<p>
					{#if signupsEnabled}
						Are you sure you want to <strong
							>disable new signups</strong
						>? New users will no longer be able to create accounts.
					{:else}
						Are you sure you want to <strong
							>enable new signups</strong
						>? New users will be able to create accounts.
					{/if}
				</p>
			</div>
			<div class="modal-footer">
				<button
					class="btn btn-secondary"
					onclick={() => (confirmingSignupToggle = false)}
					disabled={saving}
				>
					Cancel
				</button>
				<button
					class="btn btn-primary"
					onclick={confirmSignupToggle}
					disabled={saving}
				>
					{#if saving}
						Updating...
					{:else if signupsEnabled}
						Disable Signups
					{:else}
						Enable Signups
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Toast -->
{#if showToast}
	<div class="toast">{toastMessage}</div>
{/if}

<style>
	.settings-page {
		max-width: 800px;
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

	.loading {
		text-align: center;
		padding: 3rem;
		color: #64748b;
	}

	.settings-container {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.setting-card {
		background: white;
		border-radius: 8px;
		padding: 1.5rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		border: 1px solid #e2e8f0;
	}

	.setting-content {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.setting-info {
		flex: 1;
	}

	.setting-info h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #1e293b;
	}

	.setting-description {
		margin: 0;
		font-size: 0.875rem;
		color: #64748b;
		line-height: 1.4;
	}

	.setting-control {
		display: flex;
		align-items: center;
	}

	/* Toggle Switch */
	.toggle-switch {
		position: relative;
		display: inline-flex;
		width: 44px;
		height: 24px;
		flex-shrink: 0;
		cursor: pointer;
		border: none;
		border-radius: 12px;
		transition: background-color 0.2s;
		outline: none;
	}

	.toggle-switch:focus {
		box-shadow: 0 0 0 2px #3b82f6;
	}

	.toggle-switch.enabled {
		background-color: #f97316;
	}

	.toggle-switch.disabled {
		background-color: #e2e8f0;
	}

	.toggle-switch:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.toggle-slider {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 20px;
		height: 20px;
		background-color: white;
		border-radius: 50%;
		transition: transform 0.2s;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
	}

	.toggle-switch.enabled .toggle-slider {
		transform: translateX(20px);
	}

	.toggle-switch.disabled .toggle-slider {
		transform: translateX(0);
	}

	/* Select Input */
	.tier-select {
		padding: 0.5rem 0.75rem;
		border: 1px solid #e2e8f0;
		border-radius: 6px;
		font-size: 0.875rem;
		font-family: inherit;
		background: white;
		min-width: 120px;
	}

	.tier-select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 2px #3b82f6;
	}

	.tier-select:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Modal */
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
		max-width: 400px;
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
		margin: 0;
		color: #475569;
		line-height: 1.5;
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

	.btn-primary {
		background: #3b82f6;
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: #2563eb;
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

	/* Toast */
	.toast {
		position: fixed;
		bottom: 2rem;
		right: 2rem;
		background: #1e293b;
		color: white;
		padding: 1rem 1.5rem;
		border-radius: 6px;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		z-index: 1001;
		animation: slideIn 0.3s ease;
	}

	@keyframes slideIn {
		from {
			transform: translateY(100%);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	/* Responsive */
	@media (max-width: 640px) {
		.setting-content {
			flex-direction: column;
			align-items: flex-start;
			gap: 1rem;
		}

		.setting-control {
			align-self: flex-end;
		}
	}
</style>
