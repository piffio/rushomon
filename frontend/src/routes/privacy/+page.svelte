<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import Footer from "$lib/components/Footer.svelte";
	import type { PageData } from "./$types";

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Privacy Policy - Rushomon</title>
	<meta
		name="description"
		content="Privacy Policy for Rushomon URL shortener. Learn how we collect, use, and protect your personal information."
	/>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
	<Header user={data.user} />

	<main class="flex-1">
		<div class="max-w-3xl mx-auto px-6 py-12">
			<h1 class="text-4xl font-bold text-gray-900 mb-2">
				Privacy Policy
			</h1>
			<p class="text-gray-500 mb-10">Last updated: February 2026</p>

			<div class="prose prose-gray max-w-none space-y-8">
				<!-- Overview -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						1. Overview
					</h2>
					<p class="text-gray-700 leading-relaxed">
						Rushomon ("the Service") is committed to protecting your privacy.
						This Privacy Policy explains how we collect, use, and protect your
						personal information when you use our URL shortening service.
					</p>
					<p class="text-gray-700 leading-relaxed">
						This policy applies to the hosted instance of Rushomon operated at
						this domain. If you self-host Rushomon, you are responsible for
						complying with applicable privacy laws.
					</p>
				</section>

				<!-- Information We Collect -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						2. Information We Collect
					</h2>
					
					<h3 class="text-lg font-medium text-gray-800 mb-2">
						2.1 Account Information
					</h3>
					<p class="text-gray-700 leading-relaxed">
						When you create an account through OAuth providers (GitHub or Google),
						we collect the following information:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>Name:</strong> Your display name from the OAuth provider</li>
						<li><strong>Email address:</strong> Used for account identification</li>
						<li><strong>User ID:</strong> Unique identifier from the OAuth provider</li>
					</ul>
					
					<h3 class="text-lg font-medium text-gray-800 mb-2 mt-4">
						2.2 Service Data
					</h3>
					<p class="text-gray-700 leading-relaxed">
						To provide our URL shortening service, we collect and store:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>URLs:</strong> Original URLs you want to shorten</li>
						<li><strong>Short codes:</strong> Generated short codes for your links</li>
						<li><strong>Analytics data:</strong> Click counts and timestamps for your links</li>
						<li><strong>Organization data:</strong> Organization name and tier information</li>
					</ul>
					
					<h3 class="text-lg font-medium text-gray-800 mb-2 mt-4">
						2.3 Technical Data
					</h3>
					<p class="text-gray-700 leading-relaxed">
						We automatically collect technical information to operate the service:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>IP addresses:</strong> Temporarily for rate limiting and security</li>
						<li><strong>Browser information:</strong> User agent and browser type</li>
						<li><strong>Usage patterns:</strong> How you interact with our service</li>
					</ul>
				</section>

				<!-- How We Use Your Information -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						3. How We Use Your Information
					</h2>
					<p class="text-gray-700 leading-relaxed">
						We use your information for the following purposes:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>Service Provision:</strong> To create and manage your shortened URLs</li>
						<li><strong>Account Management:</strong> To authenticate and manage your account</li>
						<li><strong>Analytics:</strong> To provide click statistics for your links</li>
						<li><strong>Security:</strong> To detect and prevent abuse or malicious activity</li>
						<li><strong>Service Improvement:</strong> To analyze usage patterns and improve our service</li>
						<li><strong>Legal Compliance:</strong> To comply with applicable laws and regulations</li>
					</ul>
				</section>

				<!-- Data Sharing and Disclosure -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						4. Data Sharing and Disclosure
					</h2>
					<p class="text-gray-700 leading-relaxed">
						We do not sell, trade, or otherwise transfer your personal information to
						third parties, except in the following circumstances:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>Service Providers:</strong> We may share data with Cloudflare for hosting and infrastructure</li>
						<li><strong>Legal Requirements:</strong> We may disclose information if required by law or court order</li>
						<li><strong>Safety:</strong> We may share information to protect our rights, property, or safety</li>
						<li><strong>Business Transfer:</strong> In the event of a merger, acquisition, or sale of assets</li>
					</ul>
				</section>

				<!-- Data Security -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						5. Data Security
					</h2>
					<p class="text-gray-700 leading-relaxed">
						We implement appropriate technical and organizational measures to protect
						your personal information, including:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>Encryption:</strong> Data is encrypted in transit using HTTPS</li>
						<li><strong>Access Controls:</strong> Limited access to personal data</li>
						<li><strong>Secure Authentication:</strong> OAuth providers handle authentication</li>
						<li><strong>Regular Security Updates:</strong> We keep our systems updated and secure</li>
					</ul>
					<p class="text-gray-700 leading-relaxed mt-3">
						However, no method of transmission over the internet is 100% secure.
						While we strive to protect your data, we cannot guarantee absolute security.
					</p>
				</section>

				<!-- Data Retention -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						6. Data Retention
					</h2>
					<p class="text-gray-700 leading-relaxed">
						We retain your personal information only as long as necessary for the
						purposes outlined in this privacy policy:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>Account Data:</strong> Retained until you delete your account</li>
						<li><strong>URL Data:</strong> Retained until you delete the specific URL</li>
						<li><strong>Analytics Data:</strong> Retained for the lifetime of the URL</li>
						<li><strong>Technical Data:</strong> Retained for a limited period for security purposes</li>
					</ul>
					<p class="text-gray-700 leading-relaxed mt-3">
						You can request deletion of your account and associated data at any time
						through your account settings or by contacting us.
					</p>
				</section>

				<!-- Your Rights -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						7. Your Rights
					</h2>
					<p class="text-gray-700 leading-relaxed">
						Depending on your location, you may have the following rights regarding
						your personal information:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>Access:</strong> Request access to your personal information</li>
						<li><strong>Correction:</strong> Request correction of inaccurate information</li>
						<li><strong>Deletion:</strong> Request deletion of your personal information</li>
						<li><strong>Portability:</strong> Request transfer of your data to another service</li>
						<li><strong>Objection:</strong> Object to processing of your personal information</li>
					</ul>
					<p class="text-gray-700 leading-relaxed mt-3">
						To exercise these rights, please contact us using the information provided below.
					</p>
				</section>

				<!-- International Data Transfers -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						8. International Data Transfers
					</h2>
					<p class="text-gray-700 leading-relaxed">
						Rushomon is hosted on Cloudflare's global infrastructure. Your personal
						information may be transferred to and processed in countries other than
						your own. We ensure appropriate safeguards are in place for such transfers.
					</p>
				</section>

				<!-- Children's Privacy -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						9. Children's Privacy
					</h2>
					<p class="text-gray-700 leading-relaxed">
						Our service is not intended for children under 13 years of age. We do not
						knowingly collect personal information from children under 13. If we become
						aware that we have collected such information, we will take steps to delete it.
					</p>
				</section>

				<!-- Changes to This Policy -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						10. Changes to This Policy
					</h2>
					<p class="text-gray-700 leading-relaxed">
						We may update this Privacy Policy from time to time. We will notify you
						of any changes by posting the new policy on this page and updating the
						"Last updated" date. Continued use of the service after such changes
						constitutes acceptance of the updated policy.
					</p>
				</section>

				<!-- Contact Information -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						11. Contact Information
					</h2>
					<p class="text-gray-700 leading-relaxed">
						If you have questions about this Privacy Policy or want to exercise your
						rights, please contact us:
					</p>
					<ul class="list-disc list-inside text-gray-700 leading-relaxed ml-4">
						<li><strong>GitHub Issues:</strong> <a href="https://github.com/piffio/rushomon/issues" class="text-orange-600 hover:text-orange-700">Create an issue on GitHub</a></li>
						<li><strong>Email:</strong> Available through our GitHub repository</li>
						<li><strong>Repository:</strong> <a href="https://github.com/piffio/rushomon" class="text-orange-600 hover:text-orange-700">https://github.com/piffio/rushomon</a></li>
					</ul>
				</section>
			</div>
		</div>
	</main>

	<Footer />
</div>
