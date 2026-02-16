<script lang="ts">
	import { adminApi } from '$lib/api/admin';

	interface Props {
		linkId: string;
		linkUrl?: string;
		onClose: () => void;
	}

	let { linkId, linkUrl, onClose }: Props = $props();

	let reason = $state('');
	let reporterEmail = $state('');
	let submitting = $state(false);
	let showToast = $state(false);
	let toastMessage = $state('');
	let submitted = $state(false);

	const reportReasons = [
		{ value: 'spam', label: 'Spam or misleading content' },
		{ value: 'phishing', label: 'Phishing or malicious site' },
		{ value: 'malware', label: 'Malware or virus' },
		{ value: 'illegal', label: 'Illegal content' },
		{ value: 'harassment', label: 'Harassment or abuse' },
		{ value: 'other', label: 'Other' }
	];

	async function handleSubmit() {
		if (!reason) return;

		try {
			submitting = true;
			await adminApi.reportLink(linkId, reason, reporterEmail || undefined);
			submitted = true;
			showToastMessage('Report submitted successfully');
			setTimeout(() => {
				onClose();
			}, 2000);
		} catch (err) {
			console.error('Failed to submit report:', err);
			showToastMessage('Failed to submit report. Please try again.');
		} finally {
			submitting = false;
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

{#if !submitted}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Enter' && onClose()}
	>
		<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
			<div class="modal-header">
				<h2>Report Link</h2>
				<button class="modal-close" onclick={onClose}>&times;</button>
			</div>
			<div class="modal-body">
				<p class="description">
					Help us keep the platform safe by reporting links that violate our terms of service.
				</p>

				{#if linkUrl}
					<div class="link-preview">
						<strong>Link:</strong>
						<code>{linkUrl}</code>
					</div>
				{/if}

				<div class="form-group">
					<label for="reason">Reason for reporting *</label>
					<select id="reason" bind:value={reason} required>
						<option value="">Select a reason...</option>
						{#each reportReasons as option}
							<option value={option.value}>{option.label}</option>
						{/each}
					</select>
				</div>

				<div class="form-group">
					<label for="email">Your email (optional)</label>
					<input
						id="email"
						type="email"
						bind:value={reporterEmail}
						placeholder="email@example.com"
					/>
					<p class="hint">Provide your email if you'd like us to follow up with you.</p>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" onclick={onClose} disabled={submitting}>
					Cancel
				</button>
				<button class="btn btn-primary" onclick={handleSubmit} disabled={!reason || submitting}>
					{submitting ? 'Submitting...' : 'Submit Report'}
				</button>
			</div>
		</div>
	</div>
{:else}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Enter' && onClose()}
	>
		<div class="modal success-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
			<div class="modal-body center">
				<div class="success-icon">✓</div>
				<h3>Thank You</h3>
				<p>Your report has been submitted successfully.</p>
				<p class="hint">Our team will review it shortly.</p>
				<button class="btn btn-primary" onclick={onClose}>Close</button>
			</div>
		</div>
	</div>
{/if}

{#if showToast}
	<div class="toast">{toastMessage}</div>
{/if}

<style>
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

	.success-modal {
		max-width: 400px;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e2e8f0;
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.modal-close {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: #64748b;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.modal-body.center {
		text-align: center;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e2e8f0;
	}

	.description {
		margin: 0 0 1rem 0;
		color: #64748b;
		font-size: 0.875rem;
	}

	.link-preview {
		background: #f1f5f9;
		padding: 0.75rem;
		border-radius: 6px;
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}

	.link-preview code {
		display: block;
		margin-top: 0.25rem;
		word-break: break-all;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		font-size: 0.875rem;
		color: #475569;
	}

	.form-group select,
	.form-group input {
		width: 100%;
		padding: 0.625rem;
		border: 1px solid #e2e8f0;
		border-radius: 6px;
		font-size: 0.875rem;
		font-family: inherit;
	}

	.hint {
		margin: 0.25rem 0 0 0;
		font-size: 0.75rem;
		color: #64748b;
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

	.success-icon {
		width: 64px;
		height: 64px;
		background: #22c55e;
		color: white;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 2rem;
		margin: 0 auto 1rem;
	}

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
</style>
