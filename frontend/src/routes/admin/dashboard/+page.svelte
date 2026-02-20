<script lang="ts">
	import { onMount } from 'svelte';
	import { adminApi } from '$lib/api/admin';

	let stats = $state({
		totalUsers: 0,
		totalLinks: 0,
		totalClicks: 0,
		suspendedUsers: 0,
		blockedLinks: 0,
		blacklistedDomains: 0
	});
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		try {
			// Get basic stats
			const usersResponse = await adminApi.listUsers(1, 1); // Just get total count
			const linksResponse = await adminApi.listLinks(1, 1); // Just get total count
			const blacklistResponse = await adminApi.getBlacklist();

			stats.totalUsers = usersResponse.total;
			stats.totalLinks = linksResponse.total;
			stats.blacklistedDomains = blacklistResponse.length;

			// Note: We'd need additional endpoints for suspended users, blocked links, and total clicks
			// For now, we'll show what we can get
		} catch (err) {
			error = 'Failed to load dashboard statistics';
			console.error('Dashboard error:', err);
		} finally {
			loading = false;
		}
	});
</script>

<div class="dashboard-page">
	<div class="page-header">
		<h1>Admin Dashboard</h1>
		<p class="subtitle">Overview and system statistics</p>
	</div>

	{#if loading}
		<div class="loading">Loading dashboard...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-icon">👥</div>
				<div class="stat-content">
					<h3>{stats.totalUsers}</h3>
					<p>Total Users</p>
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-icon">🔗</div>
				<div class="stat-content">
					<h3>{stats.totalLinks}</h3>
					<p>Total Links</p>
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-icon">📊</div>
				<div class="stat-content">
					<h3>{stats.totalClicks || '—'}</h3>
					<p>Total Clicks</p>
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-icon">🚫</div>
				<div class="stat-content">
					<h3>{stats.blacklistedDomains}</h3>
					<p>Blacklisted Domains</p>
				</div>
			</div>
		</div>

		<div class="quick-actions">
			<h2>Quick Actions</h2>
			<div class="action-grid">
				<a href="/admin/users" class="action-card">
					<div class="action-icon">👥</div>
					<div class="action-content">
						<h3>Manage Users</h3>
						<p>View, suspend, or manage user roles</p>
					</div>
				</a>

				<a href="/admin/links" class="action-card">
					<div class="action-icon">🔗</div>
					<div class="action-content">
						<h3>Moderate Links</h3>
						<p>Review, block, or delete problematic links</p>
					</div>
				</a>

				<a href="/admin/blacklist" class="action-card">
					<div class="action-icon">🚫</div>
					<div class="action-content">
						<h3>Manage Blacklist</h3>
						<p>Add or remove blocked destinations</p>
					</div>
				</a>

				<a href="/admin/settings" class="action-card">
					<div class="action-icon">⚙️</div>
					<div class="action-content">
						<h3>Instance Settings</h3>
						<p>Configure system-wide settings</p>
					</div>
				</a>
			</div>
		</div>

		<div class="recent-activity">
			<h2>System Overview</h2>
			<div class="activity-grid">
				<div class="activity-item">
					<h3>🔗 Link Management</h3>
					<p>Monitor and moderate all links created on the platform</p>
					<a href="/admin/links" class="action-link">View Links →</a>
				</div>

				<div class="activity-item">
					<h3>👥 User Administration</h3>
					<p>Manage user accounts, roles, and access permissions</p>
					<a href="/admin/users" class="action-link">Manage Users →</a>
				</div>

				<div class="activity-item">
					<h3>🛡️ Content Moderation</h3>
					<p>Review abuse reports and manage blacklisted content</p>
					<a href="/admin/blacklist" class="action-link">View Blacklist →</a>
				</div>

				<div class="activity-item">
					<h3>⚙️ System Configuration</h3>
					<p>Configure instance-wide settings and policies</p>
					<a href="/admin/settings" class="action-link">Settings →</a>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.dashboard-page {
		max-width: 1200px;
		margin: 0 auto;
		padding: 0;
	}

	.page-header {
		margin-bottom: 2rem;
	}

	.page-header h1 {
		margin: 0 0 0.5rem 0;
		font-size: 2rem;
		font-weight: 600;
		color: #1e293b;
	}

	.subtitle {
		margin: 0;
		color: #64748b;
		font-size: 1rem;
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

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.stat-card {
		background: white;
		border-radius: 8px;
		padding: 1.5rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.stat-icon {
		font-size: 2rem;
		width: 60px;
		height: 60px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f1f5f9;
		border-radius: 8px;
	}

	.stat-content h3 {
		margin: 0 0 0.25rem 0;
		font-size: 1.875rem;
		font-weight: 700;
		color: #1e293b;
	}

	.stat-content p {
		margin: 0;
		color: #64748b;
		font-size: 0.875rem;
	}

	.quick-actions {
		margin-bottom: 2rem;
	}

	.quick-actions h2 {
		margin: 0 0 1rem 0;
		font-size: 1.5rem;
		font-weight: 600;
		color: #1e293b;
	}

	.action-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 1rem;
	}

	.action-card {
		background: white;
		border-radius: 8px;
		padding: 1.5rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		display: flex;
		align-items: center;
		gap: 1rem;
		text-decoration: none;
		color: inherit;
		transition: all 0.2s;
	}

	.action-card:hover {
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		transform: translateY(-2px);
	}

	.action-icon {
		font-size: 1.5rem;
		width: 50px;
		height: 50px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #f1f5f9;
		border-radius: 8px;
	}

	.action-content h3 {
		margin: 0 0 0.25rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #1e293b;
	}

	.action-content p {
		margin: 0;
		color: #64748b;
		font-size: 0.875rem;
	}

	.recent-activity h2 {
		margin: 0 0 1rem 0;
		font-size: 1.5rem;
		font-weight: 600;
		color: #1e293b;
	}

	.activity-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 1rem;
	}

	.activity-item {
		background: white;
		border-radius: 8px;
		padding: 1.5rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.activity-item h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1rem;
		font-weight: 600;
		color: #1e293b;
	}

	.activity-item p {
		margin: 0 0 1rem 0;
		color: #64748b;
		font-size: 0.875rem;
	}

	.action-link {
		color: #3b82f6;
		text-decoration: none;
		font-weight: 500;
		font-size: 0.875rem;
	}

	.action-link:hover {
		text-decoration: underline;
	}
</style>
