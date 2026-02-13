<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import Footer from "$lib/components/Footer.svelte";
	import type { PageData } from "./$types";

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Terms of Service - Rushomon</title>
	<meta
		name="description"
		content="Terms of Service and Abuse Policy for Rushomon URL shortener."
	/>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
	<Header user={data.user} />

	<main class="flex-1">
		<div class="max-w-3xl mx-auto px-6 py-12">
			<h1 class="text-4xl font-bold text-gray-900 mb-2">
				Terms of Service
			</h1>
			<p class="text-gray-500 mb-10">Last updated: February 2026</p>

			<div class="prose prose-gray max-w-none space-y-8">
				<!-- 1. Acceptance -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						1. Acceptance of Terms
					</h2>
					<p class="text-gray-700 leading-relaxed">
						By accessing or using Rushomon ("the Service"), you
						agree to be bound by these Terms of Service. If you do
						not agree to these terms, you may not use the Service.
						Rushomon is an open-source, self-hostable URL shortener.
						These terms apply to the hosted instance operated at
						this domain.
					</p>
				</section>

				<!-- 2. Description -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						2. Description of Service
					</h2>
					<p class="text-gray-700 leading-relaxed">
						Rushomon provides URL shortening and basic click
						analytics. The Service is provided "as is" without
						warranty of any kind. We reserve the right to modify,
						suspend, or discontinue the Service at any time without
						notice.
					</p>
				</section>

				<!-- 3. Accounts -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						3. User Accounts
					</h2>
					<p class="text-gray-700 leading-relaxed">
						You may sign up using a supported OAuth provider (e.g.,
						GitHub). You are responsible for maintaining the
						security of your account. You must not share your
						account credentials or allow others to access your
						account. You are responsible for all activity that
						occurs under your account.
					</p>
				</section>

				<!-- 4. Free Tier -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						4. Free Tier Limits
					</h2>
					<p class="text-gray-700 leading-relaxed mb-3">
						Free tier accounts are subject to the following monthly
						limits:
					</p>
					<ul
						class="list-disc list-inside text-gray-700 space-y-1.5 ml-2"
					>
						<li>
							<strong>25 short links</strong> per calendar month
						</li>
						<li>
							<strong>7 days</strong> of analytics data retention
						</li>
					</ul>
					<p class="text-gray-700 leading-relaxed mt-3">
						Limits reset on the 1st of each calendar month (UTC).
						The instance administrator may change your tier at any
						time. Unused quota does not roll over.
					</p>
				</section>

				<!-- 5. Acceptable Use -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						5. Acceptable Use
					</h2>
					<p class="text-gray-700 leading-relaxed mb-3">
						You agree not to use the Service to:
					</p>
					<ul
						class="list-disc list-inside text-gray-700 space-y-1.5 ml-2"
					>
						<li>
							Shorten URLs that link to malware, phishing sites,
							or other malicious content
						</li>
						<li>Distribute spam or unsolicited bulk messages</li>
						<li>
							Host or distribute illegal content under applicable
							law
						</li>
						<li>
							Engage in fraud, impersonation, or deceptive
							practices
						</li>
						<li>
							Attempt to circumvent rate limits or usage
							restrictions
						</li>
						<li>
							Interfere with or disrupt the Service or its
							infrastructure
						</li>
						<li>
							Scrape, crawl, or use automated tools to access the
							Service beyond its intended API
						</li>
					</ul>
				</section>

				<!-- 6. Abuse Policy -->
				<section id="abuse-policy">
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						6. Abuse Policy
					</h2>
					<p class="text-gray-700 leading-relaxed mb-3">
						We take abuse seriously. If we determine that your use
						of the Service violates these terms, we may take any of
						the following actions without prior notice:
					</p>
					<ul
						class="list-disc list-inside text-gray-700 space-y-1.5 ml-2"
					>
						<li>
							<strong>Disable specific links</strong> that violate
							our policies
						</li>
						<li>
							<strong>Suspend your account</strong> temporarily or
							permanently
						</li>
						<li>
							<strong>Downgrade your tier</strong> to restrict further
							usage
						</li>
						<li>
							<strong>Delete your account and data</strong> in severe
							cases
						</li>
					</ul>
					<p class="text-gray-700 leading-relaxed mt-3">
						To report abuse, please open an issue on our
						<a
							href="https://github.com/piffio/rushomon/issues"
							target="_blank"
							rel="noopener noreferrer"
							class="text-orange-600 hover:text-orange-700 underline"
							>GitHub repository</a
						>
						or contact the instance administrator.
					</p>
				</section>

				<!-- 7. Data & Privacy -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						7. Data and Privacy
					</h2>
					<p class="text-gray-700 leading-relaxed">
						Rushomon collects minimal data necessary to operate the
						Service: your OAuth profile information (name, email,
						avatar), the links you create, and basic click analytics
						(referrer, country, user agent). We do not sell your
						data to third parties. Analytics data may be subject to
						retention limits based on your tier. Data is stored on
						Cloudflare infrastructure. For self-hosted instances,
						the instance operator is responsible for data handling.
					</p>
				</section>

				<!-- 8. Intellectual Property -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						8. Intellectual Property
					</h2>
					<p class="text-gray-700 leading-relaxed">
						Rushomon is open-source software licensed under the
						<a
							href="https://github.com/piffio/rushomon/blob/main/LICENSE.md"
							target="_blank"
							rel="noopener noreferrer"
							class="text-orange-600 hover:text-orange-700 underline"
							>GNU Affero General Public License v3.0</a
						>. You retain ownership of the content you create
						through the Service (your links and associated
						metadata).
					</p>
				</section>

				<!-- 9. Limitation of Liability -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						9. Limitation of Liability
					</h2>
					<p class="text-gray-700 leading-relaxed">
						The Service is provided "as is" and "as available"
						without warranties of any kind, either express or
						implied. We are not liable for any damages arising from
						your use of the Service, including but not limited to
						loss of data, loss of revenue, or service interruptions.
						Short links may stop working if the Service is
						discontinued or your account is terminated.
					</p>
				</section>

				<!-- 10. Changes -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						10. Changes to Terms
					</h2>
					<p class="text-gray-700 leading-relaxed">
						We may update these Terms of Service from time to time.
						Continued use of the Service after changes constitutes
						acceptance of the updated terms. Material changes will
						be communicated through the Service or via the
						repository.
					</p>
				</section>

				<!-- 11. Contact -->
				<section>
					<h2 class="text-2xl font-semibold text-gray-900 mb-3">
						11. Contact
					</h2>
					<p class="text-gray-700 leading-relaxed">
						For questions about these terms, please open an issue on
						our
						<a
							href="https://github.com/piffio/rushomon/issues"
							target="_blank"
							rel="noopener noreferrer"
							class="text-orange-600 hover:text-orange-700 underline"
							>GitHub repository</a
						>
						or contact the instance administrator.
					</p>
				</section>
			</div>
		</div>
	</main>

	<Footer />
</div>
