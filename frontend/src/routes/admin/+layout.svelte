<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { authApi } from "$lib/api/auth";
    import UserMenu from "$lib/components/UserMenu.svelte";
    import Avatar from "$lib/components/Avatar.svelte";
    import type { User } from "$lib/types/api";

    type Module =
        | "dashboard"
        | "users"
        | "billing"
        | "links"
        | "blacklist"
        | "reports"
        | "api-keys"
        | "settings";

    let { children } = $props();

    const activeModule = $derived(
        page.url.pathname.split("/").pop() || "dashboard"
    );

    function navigateTo(module: Module) {
        goto(`/admin/${module}`);
    }

    let sidebarCollapsed = $state(true); // Collapsed by default on desktop
    let mobileMenuOpen = $state(false);
    let currentUser = $state<User | null>(null);
    const isOnDashboard = $derived(page.url.pathname === "/dashboard");

    onMount(async () => {
        try {
            currentUser = await authApi.me();
        } catch (error) {
            console.error("Failed to load user:", error);
        }
    });

    async function handleLogout() {
        try {
            await authApi.logout();
            window.location.href = "/";
        } catch (error) {
            console.error("Logout failed:", error);
            window.location.href = "/";
        }
    }
</script>

<div class="admin-layout">
    <aside class="sidebar" class:collapsed={sidebarCollapsed}>
        <div class="sidebar-header">
            <button
                class="collapse-btn"
                onclick={() => (sidebarCollapsed = !sidebarCollapsed)}
                title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
                aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
                {#if sidebarCollapsed}
                    <!-- Expand arrow (points right) -->
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
                            d="M9 5l7 7-7 7"
                        />
                    </svg>
                {:else}
                    <!-- Collapse arrow (points left) -->
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
                            d="M15 19l-7-7 7-7"
                        />
                    </svg>
                {/if}
            </button>
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
                class="nav-item {activeModule === 'billing' ? 'active' : ''}"
                onclick={() => navigateTo("billing")}
            >
                <span class="nav-icon">💳</span>
                <span class="nav-label">Billing Accounts</span>
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
                class="nav-item {activeModule === 'api-keys' ? 'active' : ''}"
                onclick={() => navigateTo("api-keys")}
            >
                <span class="nav-icon">🔑</span>
                <span class="nav-label">API Keys</span>
            </button>
            <button
                class="nav-item {activeModule === 'settings' ? 'active' : ''}"
                onclick={() => navigateTo("settings")}
            >
                <span class="nav-icon">⚙️</span>
                <span class="nav-label">Instance Settings</span>
            </button>
            <a
                href="/dashboard"
                class="nav-item back-link"
                title="Go to your Dashboard"
            >
                <span class="nav-icon">📊</span>
                <span class="nav-label">Dashboard</span>
            </a>
        </nav>
    </aside>
    <main class="main-content" class:sidebar-collapsed={sidebarCollapsed}>
        <div class="top-bar">
            <button
                class="hamburger"
                onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
                aria-label="Toggle menu"
            >
                <svg
                    class="w-6 h-6"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    {#if mobileMenuOpen}
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M6 18L18 6M6 6l12 12"
                        />
                    {:else}
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M4 6h16M4 12h16M4 18h16"
                        />
                    {/if}
                </svg>
            </button>
            {#if currentUser}
                <div class="user-menu-container">
                    <UserMenu user={currentUser} onLogout={handleLogout} />
                </div>
            {/if}
        </div>

        <!-- Mobile Menu -->
        {#if mobileMenuOpen}
            <nav class="mobile-menu">
                <!-- User Info Header -->
                {#if currentUser}
                    <div class="mobile-user-info">
                        <Avatar user={currentUser} size="lg" />
                        <div class="mobile-user-details">
                            <div class="mobile-user-name">
                                {currentUser.name || "User"}
                            </div>
                            <div class="mobile-user-email">
                                {currentUser.email}
                            </div>
                        </div>
                    </div>
                {/if}

                <!-- User Menu Section -->
                <a
                    href="/settings"
                    class="mobile-nav-item"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">⚙️</span>
                    <span>Account Settings</span>
                </a>
                <a
                    href="https://github.com/piffio/rushomon/"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="mobile-nav-item"
                >
                    <span class="mobile-nav-icon">📖</span>
                    <span>Documentation</span>
                </a>
                <a
                    href="/dashboard"
                    class="mobile-nav-item"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">📊</span>
                    <span>Dashboard</span>
                </a>

                <!-- Divider -->
                <div class="mobile-nav-divider"></div>

                <!-- Admin Navigation -->
                <a
                    href="/admin/dashboard"
                    class="mobile-nav-item {activeModule === 'dashboard' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">🏠</span>
                    <span>Admin Dashboard</span>
                </a>
                <a
                    href="/admin/users"
                    class="mobile-nav-item {activeModule === 'users' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">👥</span>
                    <span>Users</span>
                </a>
                <a
                    href="/admin/billing"
                    class="mobile-nav-item {activeModule === 'billing' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">💳</span>
                    <span>Billing Accounts</span>
                </a>
                <a
                    href="/admin/links"
                    class="mobile-nav-item {activeModule === 'links' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">🔗</span>
                    <span>Links</span>
                </a>
                <a
                    href="/admin/blacklist"
                    class="mobile-nav-item {activeModule === 'blacklist' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">🚫</span>
                    <span>Blacklist</span>
                </a>
                <a
                    href="/admin/reports"
                    class="mobile-nav-item {activeModule === 'reports' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">🚨</span>
                    <span>Reports</span>
                </a>
                <a
                    href="/admin/api-keys"
                    class="mobile-nav-item {activeModule === 'api-keys' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">🔑</span>
                    <span>API Keys</span>
                </a>
                <a
                    href="/admin/settings"
                    class="mobile-nav-item {activeModule === 'settings' ? 'active' : ''}"
                    onclick={() => (mobileMenuOpen = false)}
                >
                    <span class="mobile-nav-icon">⚙️</span>
                    <span>Instance Settings</span>
                </a>

                <!-- Logout -->
                <div class="mobile-nav-divider"></div>
                <button
                    class="mobile-nav-item mobile-logout"
                    onclick={() => {
                        handleLogout();
                        mobileMenuOpen = false;
                    }}
                >
                    <span class="mobile-nav-icon">🚪</span>
                    <span>Log out</span>
                </button>
            </nav>
        {/if}

        {@render children()}
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
        transition:
            width 0.3s ease,
            transform 0.3s ease;
        z-index: 100;
    }

    .sidebar.collapsed {
        width: 60px;
    }

    .sidebar.collapsed .nav-label,
    .sidebar.collapsed .sidebar-header h2 {
        display: none;
    }

    .sidebar.collapsed .sidebar-header {
        padding: 0.75rem 0.5rem;
        justify-content: center;
    }

    .sidebar.collapsed .sidebar-header .collapse-btn {
        display: flex;
    }

    .sidebar.collapsed .nav-item {
        justify-content: center;
        padding: 0.75rem 0.5rem;
    }

    .sidebar.collapsed .nav-icon {
        margin: 0;
    }

    .sidebar-header {
        padding: 1rem;
        border-bottom: 1px solid #334155;
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .sidebar-header h2 {
        margin: 0;
        font-size: 1.25rem;
        font-weight: 600;
        white-space: nowrap;
        overflow: hidden;
        flex: 1;
    }

    .sidebar-header .collapse-btn {
        background: none;
        border: none;
        color: #94a3b8;
        cursor: pointer;
        font-size: 1.25rem;
        padding: 0.5rem;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
        flex-shrink: 0;
    }

    .sidebar-header .collapse-btn:hover {
        color: white;
    }

    .sidebar.collapsed .sidebar-header {
        padding: 0.75rem 0.5rem;
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
        white-space: nowrap;
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
        min-width: 20px;
    }

    .nav-label {
        flex: 1;
    }

    .nav-item.back-link {
        color: #94a3b8;
        border-top: 1px solid #334155;
        margin-top: 0.75rem;
        padding-top: 0.75rem;
    }

    .nav-item.back-link:hover {
        background: #334155;
        color: white;
    }

    .sidebar.collapsed .nav-item.back-link {
        padding: 0.75rem 0.5rem;
        justify-content: center;
        margin-top: 0.75rem;
        border-top: 1px solid #334155;
    }

    .main-content {
        flex: 1;
        margin-left: 250px;
        padding: 2rem;
        transition: margin-left 0.3s ease;
    }

    .main-content.sidebar-collapsed {
        margin-left: 60px;
    }

    .top-bar {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2rem;
    }

    .user-menu-container {
        margin-left: auto;
    }

    @media (max-width: 768px) {
        .user-menu-container {
            display: none;
        }
    }

    .hamburger {
        display: none;
        position: fixed;
        top: 1rem;
        right: 1rem;
        z-index: 101;
        background: #1e293b;
        color: white;
        border: none;
        padding: 0.5rem;
        border-radius: 0.5rem;
        cursor: pointer;
        transition: all 0.2s;
    }

    .hamburger:hover {
        background: #334155;
    }

    /* Mobile Menu */
    .mobile-menu {
        display: none;
        position: fixed;
        top: 4rem;
        right: 1rem;
        left: 1rem;
        background: white;
        border-radius: 0.5rem;
        box-shadow:
            0 4px 6px -1px rgba(0, 0, 0, 0.1),
            0 2px 4px -1px rgba(0, 0, 0, 0.06);
        padding: 0.5rem;
        z-index: 100;
        border: 1px solid #e2e8f0;
    }

    /* Mobile User Info Header */
    .mobile-user-info {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 1rem;
        border-bottom: 1px solid #e2e8f0;
        margin-bottom: 0.5rem;
    }

    .mobile-user-details {
        flex: 1;
        min-width: 0;
    }

    .mobile-user-name {
        font-weight: 600;
        color: #1e293b;
        font-size: 0.95rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .mobile-user-email {
        color: #64748b;
        font-size: 0.875rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .mobile-nav-divider {
        height: 1px;
        background: #e2e8f0;
        margin: 0.75rem 0;
    }

    .mobile-nav-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.75rem 1rem;
        color: #475569;
        text-decoration: none;
        border-radius: 0.375rem;
        transition: all 0.2s;
        font-size: 0.95rem;
    }

    .mobile-nav-item:hover {
        background: #f1f5f9;
        color: #1e293b;
    }

    .mobile-nav-item.active {
        background: #dbeafe;
        color: #2563eb;
    }

    .mobile-nav-icon {
        font-size: 1.25rem;
    }

    .mobile-logout {
        width: 100%;
        text-align: left;
        background: none;
        border: none;
        cursor: pointer;
        color: #dc2626;
    }

    .mobile-logout:hover {
        background: #fee2e2;
        color: #991b1b;
    }

    @media (max-width: 768px) {
        .sidebar {
            display: none;
        }

        .main-content {
            margin-left: 0;
            padding: 1rem;
        }

        .main-content.sidebar-collapsed {
            margin-left: 0;
        }

        .top-bar {
            margin-bottom: 1rem;
        }

        .hamburger {
            display: flex;
            align-items: center;
            justify-content: center;
        }

        .mobile-menu {
            display: block;
        }
    }
</style>
