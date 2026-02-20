<script lang="ts">
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";

	type Module =
		| "dashboard"
		| "users"
		| "links"
		| "blacklist"
		| "reports"
		| "settings";

	$: activeModule = $page.url.pathname.split("/").pop() || "dashboard";

	function navigateTo(module: Module) {
		goto(`/admin/${module}`);
	}
</script>

<div class="admin-layout">
	<aside class="sidebar">
		<div class="sidebar-header">
			<h2>Admin Console</h2>
		</div>
		<nav class="sidebar-nav">
			<button
				class="nav-item {activeModule === 'dashboard' ? 'active' : ''}"
				onclick={() => navigateTo("dashboard")}
			>
				<span class="nav-icon">🏠</span>
				<span class="nav-label">Dashboard</span>
			</button>
			<button
				class="nav-item {activeModule === 'users' ? 'active' : ''}"
				onclick={() => navigateTo("users")}
			>
				<span class="nav-icon">👥</span>
				<span class="nav-label">Users</span>
			</button>
			<button
				class="nav-item {activeModule === 'links' ? 'active' : ''}"
				onclick={() => navigateTo("links")}
			>
				<span class="nav-icon">🔗</span>
				<span class="nav-label">Links</span>
			</button>
			<button
				class="nav-item {activeModule === 'blacklist' ? 'active' : ''}"
				onclick={() => navigateTo("blacklist")}
			>
				<span class="nav-icon">🚫</span>
				<span class="nav-label">Blacklist</span>
			</button>
			<button
				class="nav-item {activeModule === 'reports' ? 'active' : ''}"
				onclick={() => navigateTo("reports")}
			>
				<span class="nav-icon">🚨</span>
				<span class="nav-label">Reports</span>
			</button>
			<button
				class="nav-item {activeModule === 'settings' ? 'active' : ''}"
				onclick={() => navigateTo("settings")}
			>
				<span class="nav-icon">⚙️</span>
				<span class="nav-label">Instance Settings</span>
			</button>
		</nav>
	</aside>
	<main class="main-content">
		<slot />
	</main>
</div>

<style>
	.admin-layout {
		display: flex;
		min-height: 100vh;
	}

	.sidebar {
		width: 250px;
		background: #1e293b;
		color: white;
		display: flex;
		flex-direction: column;
		position: fixed;
		height: 100vh;
	}

	.sidebar-header {
		padding: 1.5rem;
		border-bottom: 1px solid #334155;
	}

	.sidebar-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.sidebar-nav {
		flex: 1;
		padding: 1rem 0;
	}

	.nav-item {
		width: 100%;
		padding: 0.75rem 1.5rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		background: none;
		border: none;
		color: #94a3b8;
		cursor: pointer;
		text-align: left;
		font-size: 0.95rem;
		transition: all 0.2s;
	}

	.nav-item:hover {
		background: #334155;
		color: white;
	}

	.nav-item.active {
		background: #3b82f6;
		color: white;
	}

	.nav-icon {
		font-size: 1.25rem;
	}

	.nav-label {
		flex: 1;
	}

	.main-content {
		flex: 1;
		margin-left: 250px;
		padding: 2rem;
	}
</style>
