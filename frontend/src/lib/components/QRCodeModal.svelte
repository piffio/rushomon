<script lang="ts">
	import { onMount } from "svelte";
	import QRCode from "qrcode";
	import type { Link } from "$lib/types/api";

	interface Props {
		link: Link | null;
		isOpen: boolean;
		onClose: () => void;
	}

	let { link, isOpen, onClose }: Props = $props();

	let canvas: HTMLCanvasElement = $state() as HTMLCanvasElement;
	let shortUrl: string = $state("");
	let downloadLink: HTMLAnchorElement | null = null;

	// Generate QR code when modal opens
	$effect(() => {
		if (isOpen && link && canvas) {
			shortUrl = `${window.location.origin.replace("localhost:8787", "localhost:5173")}/${link.short_code}`;
			generateQR();
		}
	});

	async function generateQR() {
		if (!canvas || !shortUrl) return;

		try {
			await QRCode.toCanvas(canvas, shortUrl, {
				width: 256,
				margin: 2,
				color: {
					dark: "#000000",
					light: "#FFFFFF",
				},
			});
		} catch (err) {
			console.error("Error generating QR code:", err);
		}
	}

	function downloadPNG() {
		if (!canvas || !link) return;

		const anchor = document.createElement("a");
		anchor.download = `${link.short_code}-qr.png`;
		anchor.href = canvas.toDataURL("image/png");
		anchor.click();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			onClose();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && link}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-50 overflow-y-auto"
		aria-labelledby="qr-modal-title"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		onclick={(e) => {
			if (e.target === e.currentTarget) onClose();
		}}
	>
		<!-- Backdrop -->
		<div class="fixed inset-0 bg-gray-900/50 transition-opacity"></div>

		<div class="flex min-h-full items-center justify-center p-4">
			<div
				class="relative transform overflow-hidden rounded-lg bg-white shadow-xl transition-all max-w-md w-full"
			>
				<!-- Header -->
				<div
					class="flex items-center justify-between px-6 py-4 border-b border-gray-200"
				>
					<h3
						id="qr-modal-title"
						class="text-lg font-semibold text-gray-900"
					>
						QR Code
					</h3>
					<button
						onclick={onClose}
						class="text-gray-400 hover:text-gray-600 transition-colors p-1 rounded-full hover:bg-gray-100"
						aria-label="Close"
					>
						<svg
							class="w-5 h-5"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							></path>
						</svg>
					</button>
				</div>

				<!-- Content -->
				<div class="px-6 py-6">
					{#if link.title}
						<p
							class="text-sm text-gray-600 mb-4 font-medium truncate"
						>
							{link.title}
						</p>
					{/if}

					<!-- QR Code Canvas -->
					<div class="flex justify-center mb-6">
						<canvas
							bind:this={canvas}
							class="border border-gray-200 rounded-lg"
							width="256"
							height="256"
						></canvas>
					</div>

					<!-- Short URL -->
					<p
						class="text-center text-sm text-gray-600 mb-6 font-mono break-all"
					>
						{shortUrl}
					</p>

					<!-- Actions -->
					<div class="flex gap-3">
						<button
							onclick={downloadPNG}
							class="flex-1 px-4 py-2.5 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-medium hover:from-orange-600 hover:to-orange-700 transition-all flex items-center justify-center gap-2"
						>
							<svg
								class="w-4 h-4"
								fill="none"
								stroke="currentColor"
								viewBox="0 0 24 24"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
								></path>
							</svg>
							Download PNG
						</button>
						<button
							onclick={onClose}
							class="px-4 py-2.5 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
						>
							Close
						</button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
