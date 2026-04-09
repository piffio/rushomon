<script lang="ts">
  import type { BillingStatus } from "$lib/api/billing";
  import { billingApi } from "$lib/api/billing";
  import { resolveLogoUrl } from "$lib/api/client";
  import { tagsApi } from "$lib/api/links";
  import { orgsApi } from "$lib/api/orgs";
  import Avatar from "$lib/components/Avatar.svelte";
  import LoadingButton from "$lib/components/LoadingButton.svelte";
  import type {
    OrgDetails,
    OrgInvitation,
    OrgMember,
    OrgSettings,
    OrgWithRole,
    TagWithCount
  } from "$lib/types/api";
  import type { PageData } from "./$types";

  const { data }: { data: PageData } = $props();

  let orgDetails = $state<OrgDetails | null>(null);
  let loading = $state(true);
  let error = $state("");
  let billingStatus = $state<BillingStatus | null>(null);

  // Rename org
  let editingName = $state(false);
  let newOrgName = $state("");
  let savingName = $state(false);
  let nameError = $state("");

  // Invite member
  let inviteEmail = $state("");
  let inviting = $state(false);
  let inviteError = $state("");
  let inviteSuccess = $state("");

  // General feedback
  let actionError = $state("");
  let actionSuccess = $state("");

  // Tags management
  let tags = $state<TagWithCount[]>([]);
  let tagsLoading = $state(false);
  let tagsError = $state("");
  let editingTag = $state<string | null>(null);
  let newTagName = $state("");
  let savingTag = $state(false);
  let tagError = $state("");
  let deletingTag = $state<string | null>(null);

  async function loadOrg() {
    loading = true;
    error = "";
    try {
      const orgsRes = await orgsApi.listMyOrgs();
      const currentOrgId = orgsRes.current_org_id;
      if (!currentOrgId) {
        error = "No active organization found.";
        return;
      }
      orgDetails = await orgsApi.getOrg(currentOrgId);
      await loadOrgSettings(currentOrgId);
      await loadTags();
    } catch (e: unknown) {
      error =
        e instanceof Error ? e.message : "Failed to load organization details.";
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadOrg();
    billingApi
      .getStatus()
      .then((s) => {
        billingStatus = s;
      })
      .catch(() => {});
  });

  function startEditName() {
    if (!orgDetails) return;
    newOrgName = orgDetails.org.name;
    editingName = true;
    nameError = "";
  }

  async function saveOrgName() {
    if (!orgDetails) return;
    const trimmed = newOrgName.trim();
    if (!trimmed) {
      nameError = "Name cannot be empty.";
      return;
    }
    if (trimmed.length > 100) {
      nameError = "Name must be 100 characters or less.";
      return;
    }
    savingName = true;
    nameError = "";
    try {
      await orgsApi.updateOrgName(orgDetails.org.id, trimmed);
      orgDetails.org.name = trimmed;
      editingName = false;
      actionSuccess = "Organization name updated.";
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      if (e instanceof Error) {
        nameError = e.message;
      } else {
        nameError = "Failed to save name.";
      }
    } finally {
      savingName = false;
    }
  }

  async function handleInvite() {
    if (!orgDetails) return;
    const email = inviteEmail.trim().toLowerCase();
    if (!email || !email.includes("@")) {
      inviteError = "Please enter a valid email address.";
      return;
    }
    inviting = true;
    inviteError = "";
    inviteSuccess = "";
    try {
      await orgsApi.inviteMember(orgDetails.org.id, email);
      inviteSuccess = `Invitation sent to ${email}.`;
      inviteEmail = "";
      await loadOrg();
    } catch (e: unknown) {
      if (e instanceof Error) {
        inviteError = e.message;
      } else {
        inviteError = "Failed to send invitation.";
      }
    } finally {
      inviting = false;
    }
  }

  async function handleRevokeInvitation(inv: OrgInvitation) {
    if (!orgDetails) return;
    actionError = "";
    actionSuccess = "";
    try {
      await orgsApi.revokeInvitation(orgDetails.org.id, inv.id);
      actionSuccess = `Invitation to ${inv.email} revoked.`;
      await loadOrg();
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      if (e instanceof Error) {
        actionError = e.message;
      } else {
        actionError = "Failed to revoke invitation.";
      }
    }
  }

  async function handleResendInvitation(inv: OrgInvitation) {
    if (!orgDetails) return;
    actionError = "";
    actionSuccess = "";
    try {
      await orgsApi.resendInvitation(orgDetails.org.id, inv.id);
      actionSuccess = `Invitation resent to ${inv.email}.`;
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      if (e instanceof Error) {
        actionError = e.message;
      } else {
        actionError = "Failed to resend invitation.";
      }
    }
  }

  async function handleRemoveMember(member: OrgMember) {
    if (!orgDetails) return;
    confirmingRemoveMember = member;
  }

  async function confirmRemoveMember() {
    if (!confirmingRemoveMember) return;
    actionError = "";
    actionSuccess = "";
    const memberEmail = confirmingRemoveMember.email;
    try {
      await orgsApi.removeMember(
        orgDetails!.org.id,
        confirmingRemoveMember.user_id
      );
      actionSuccess = `${memberEmail} removed from the organization.`;
      await loadOrg();
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      if (e instanceof Error) {
        actionError = e.message;
      } else {
        actionError = "Failed to remove member.";
      }
    } finally {
      confirmingRemoveMember = null;
    }
  }

  function cancelRemoveMember() {
    confirmingRemoveMember = null;
  }

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric"
    });
  }

  const isOwner = $derived(orgDetails?.org.role === "owner");
  const isPro = $derived(
    ["pro", "business", "unlimited"].includes(orgDetails?.org.tier ?? "")
  );
  // Business tier only allows team members (up to 20)
  const canInviteMembers = $derived(
    ["business", "unlimited"].includes(orgDetails?.org.tier ?? "")
  );

  // Org settings: forward_query_params
  let orgSettings = $state<OrgSettings | null>(null);
  let settingsError = $state("");
  let settingsSaving = $state(false);

  async function loadOrgSettings(orgId: string) {
    try {
      orgSettings = await orgsApi.getOrgSettings(orgId);
    } catch {
      // Non-critical
    }
  }

  async function loadTags() {
    tagsLoading = true;
    tagsError = "";
    try {
      tags = await tagsApi.list();
    } catch (e: unknown) {
      if (e instanceof Error) {
        tagsError = e.message;
      } else {
        tagsError = "Failed to load tags.";
      }
    } finally {
      tagsLoading = false;
    }
  }

  async function toggleForwardQueryParams(value: boolean) {
    if (!orgDetails) return;
    settingsSaving = true;
    settingsError = "";
    try {
      orgSettings = await orgsApi.updateOrgSettings(orgDetails.org.id, {
        forward_query_params: value
      });
      actionSuccess = "Organization settings updated.";
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      if (e instanceof Error) {
        settingsError = e.message;
      } else {
        settingsError = "Failed to update settings.";
      }
    } finally {
      settingsSaving = false;
    }
  }

  // Logo management
  let logoUploading = $state(false);
  let logoDeleting = $state(false);
  let logoError = $state("");
  let logoFileInput = $state<HTMLInputElement | undefined>(undefined);

  async function handleLogoUpload(event: Event) {
    if (!orgDetails) return;
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    const allowedTypes = [
      "image/png",
      "image/jpeg",
      "image/webp",
      "image/svg+xml"
    ];
    if (!allowedTypes.includes(file.type)) {
      logoError = "Invalid file type. Allowed: PNG, JPEG, WebP, SVG.";
      return;
    }
    if (file.size > 500 * 1024) {
      logoError = "File must be 500 KB or smaller.";
      return;
    }

    logoUploading = true;
    logoError = "";
    try {
      const result = await orgsApi.uploadOrgLogo(orgDetails.org.id, file);
      orgDetails.org.logo_url = result.logo_url;
      actionSuccess = "Logo uploaded successfully.";
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      if (e instanceof Error) {
        logoError = e.message;
      } else {
        logoError = "Failed to upload logo.";
      }
    } finally {
      logoUploading = false;
      input.value = "";
    }
  }

  async function handleDeleteLogo() {
    if (!orgDetails) return;
    logoDeleting = true;
    logoError = "";
    try {
      await orgsApi.deleteOrgLogo(orgDetails.org.id);
      orgDetails.org.logo_url = null;
      actionSuccess = "Logo removed.";
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      if (e instanceof Error) {
        logoError = e.message;
      } else {
        logoError = "Failed to remove logo.";
      }
    } finally {
      logoDeleting = false;
    }
  }

  // Delete organization
  let showDeleteModal = $state(false);
  let deleteAction = $state<"delete" | "migrate">("delete");
  let targetOrgId = $state("");
  let deleting = $state(false);
  let deleteError = $state("");
  let confirmingRemoveMember = $state<OrgMember | null>(null);
  let linkCount = $state(0);
  let userOrgs = $state<OrgWithRole[]>([]);
  let canDelete = $state(false);

  async function checkCanDelete() {
    if (!orgDetails) return;
    try {
      const orgsRes = await orgsApi.listMyOrgs();
      const ownedOrgs = orgsRes.orgs.filter((o) => o.role === "owner");
      userOrgs = ownedOrgs.filter((o) => o.id !== orgDetails!.org.id);
      canDelete = ownedOrgs.length > 1;

      // Get link count from usage API
      const usage = await orgsApi.getUsage();
      linkCount = usage.usage?.links_created_this_month || 0;
    } catch {
      canDelete = false;
    }
  }

  async function openDeleteModal() {
    await checkCanDelete();
    if (!canDelete) {
      actionError = "Cannot delete your only organization.";
      setTimeout(() => (actionError = ""), 3000);
      return;
    }
    showDeleteModal = true;
    deleteError = "";
    deleteAction = "delete";
    targetOrgId = userOrgs.length > 0 ? userOrgs[0].id : "";
  }

  async function handleDeleteOrg() {
    if (!orgDetails) return;
    deleting = true;
    deleteError = "";
    try {
      await orgsApi.deleteOrg(
        orgDetails.org.id,
        deleteAction,
        deleteAction === "migrate" ? targetOrgId : undefined
      );
      // Redirect to dashboard after successful deletion
      window.location.href = "/dashboard";
    } catch (e: unknown) {
      if (e instanceof Error) {
        deleteError = e.message;
      } else {
        deleteError = "Failed to delete organization.";
      }
    } finally {
      deleting = false;
    }
  }

  // Tag management functions
  function startEditTag(tagName: string) {
    editingTag = tagName;
    newTagName = tagName;
    tagError = "";
  }

  function cancelEditTag() {
    editingTag = null;
    newTagName = "";
    tagError = "";
  }

  async function saveTagRename(oldName: string) {
    if (!orgDetails) return;
    const trimmed = newTagName.trim();
    if (!trimmed) {
      tagError = "Tag name cannot be empty.";
      return;
    }
    if (trimmed === oldName) {
      cancelEditTag();
      return;
    }
    if (trimmed.length > 50) {
      tagError = "Tag name must be 50 characters or less.";
      return;
    }

    savingTag = true;
    tagError = "";
    try {
      tags = await tagsApi.rename(oldName, trimmed);
      actionSuccess = `Tag renamed from "${oldName}" to "${trimmed}".`;
      setTimeout(() => (actionSuccess = ""), 3000);
      cancelEditTag();
    } catch (e: unknown) {
      if (e instanceof Error) {
        tagError = e.message;
      } else {
        tagError = "Failed to rename tag.";
      }
    } finally {
      savingTag = false;
    }
  }

  function startDeleteTag(tagName: string) {
    deletingTag = tagName;
  }

  function cancelDeleteTag() {
    deletingTag = null;
  }

  async function confirmDeleteTag(tagName: string) {
    if (!orgDetails) return;
    try {
      await tagsApi.remove(tagName);
      tags = tags.filter((t) => t.name !== tagName);
      actionSuccess = `Tag "${tagName}" deleted successfully.`;
      setTimeout(() => (actionSuccess = ""), 3000);
      cancelDeleteTag();
    } catch (e: unknown) {
      if (e instanceof Error) {
        actionError = e.message;
      } else {
        actionError = "Failed to delete tag.";
      }
      setTimeout(() => (actionError = ""), 3000);
    }
  }

  // Helper function for tag colors (same as SearchFilterBar)
  const TAG_COLORS = [
    "bg-blue-100 text-blue-800",
    "bg-green-100 text-green-800",
    "bg-purple-100 text-purple-800",
    "bg-yellow-100 text-yellow-800",
    "bg-pink-100 text-pink-800",
    "bg-indigo-100 text-indigo-800",
    "bg-orange-100 text-orange-800",
    "bg-teal-100 text-teal-800"
  ];

  function tagColor(tag: string): string {
    let hash = 0;
    for (let i = 0; i < tag.length; i++) {
      hash = (hash * 31 + tag.charCodeAt(i)) & 0xffffffff;
    }
    return TAG_COLORS[Math.abs(hash) % TAG_COLORS.length];
  }
</script>

<div class="min-h-screen bg-gray-50">
  <main class="container mx-auto px-4 py-8 max-w-3xl">
    <div class="mb-6">
      <h1 class="text-2xl font-bold text-gray-900">Organization Settings</h1>
    </div>

    {#if loading}
      <div class="flex items-center justify-center py-16">
        <div
          class="w-8 h-8 border-2 border-orange-500 border-t-transparent rounded-full animate-spin"
        ></div>
      </div>
    {:else if error}
      <div class="bg-red-50 border border-red-200 rounded-xl p-4 text-red-700">
        {error}
      </div>
    {:else if orgDetails}
      <!-- Global feedback -->
      {#if actionSuccess}
        <div
          class="mb-4 bg-green-50 border border-green-200 rounded-xl p-3 text-green-700 text-sm"
        >
          {actionSuccess}
        </div>
      {/if}
      {#if actionError}
        <div
          class="mb-4 bg-red-50 border border-red-200 rounded-xl p-3 text-red-700 text-sm"
        >
          {actionError}
        </div>
      {/if}

      <!-- Org Name Card -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <h2 class="text-lg font-semibold text-gray-900 mb-4">
          Organization Name
        </h2>

        {#if editingName}
          <div class="flex items-center gap-3">
            <input
              type="text"
              bind:value={newOrgName}
              maxlength="100"
              class="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
              onkeydown={(e) => e.key === "Enter" && saveOrgName()}
            />
            <LoadingButton
              onclick={saveOrgName}
              loading={savingName}
              variant="primary"
            >
              {savingName ? "Saving…" : "Save"}
            </LoadingButton>
            <button
              onclick={() => (editingName = false)}
              class="px-4 py-2 text-gray-600 hover:text-gray-900 border border-gray-300 rounded-lg text-sm transition-colors"
            >
              Cancel
            </button>
          </div>
          {#if nameError}
            <p class="mt-2 text-sm text-red-600">{nameError}</p>
          {/if}
        {:else}
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span
                class="w-10 h-10 rounded-lg bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-lg font-bold"
              >
                {orgDetails.org.name.charAt(0).toUpperCase()}
              </span>
              <div>
                <p class="font-semibold text-gray-900">
                  {orgDetails.org.name}
                </p>
                <p class="text-xs text-gray-500 capitalize">
                  {orgDetails.org.tier} plan · Created {formatDate(
                    orgDetails.org.created_at
                  )}
                </p>
              </div>
            </div>
            {#if isOwner}
              <button
                onclick={startEditName}
                class="text-sm text-orange-600 hover:text-orange-700 font-medium transition-colors"
              >
                Rename
              </button>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Pro Features / org settings card -->
      {#if isPro}
        <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
          <h2 class="text-lg font-semibold text-gray-900 mb-1">
            Link Defaults
          </h2>
          <p class="text-sm text-gray-500 mb-4">
            Default settings applied to new and edited links. Changes here do
            not retroactively update existing links.
          </p>

          <div class="flex items-start gap-4 py-3 border-t border-gray-100">
            <div class="flex-1">
              <label
                for="org-forward-query-params"
                class="block text-sm font-medium text-gray-900"
              >
                Forward visitor query parameters by default
              </label>
              <p class="text-xs text-gray-500 mt-0.5">
                When enabled, new links will forward visitor query params to the
                destination URL by default. Can be overridden per link.
              </p>
            </div>
            <div class="flex items-center gap-2">
              {#if settingsSaving}
                <svg
                  class="animate-spin w-4 h-4 text-gray-400"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    class="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    stroke-width="4"
                  ></circle>
                  <path
                    class="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  ></path>
                </svg>
              {/if}
              <input
                type="checkbox"
                id="org-forward-query-params"
                checked={orgSettings?.forward_query_params ?? false}
                disabled={settingsSaving || !isOwner}
                onchange={(e) =>
                  toggleForwardQueryParams(
                    (e.target as HTMLInputElement).checked
                  )}
                class="h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500 disabled:opacity-50"
              />
            </div>
          </div>

          {#if settingsError}
            <p class="mt-2 text-sm text-red-600">{settingsError}</p>
          {/if}
        </div>
      {/if}

      <!-- Org Logo Card (Pro+) -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <h2 class="text-lg font-semibold text-gray-900 mb-1">
          Organization Logo
        </h2>
        <p class="text-sm text-gray-500 mb-4">
          Used in QR codes and branding. PNG, JPEG, WebP or SVG — max 500 KB.
        </p>

        {#if !isPro}
          <div
            class="bg-amber-50 border border-amber-200 rounded-lg p-3 text-sm text-amber-800"
          >
            Custom org logo requires a <strong>Pro plan</strong> or above.
            <a href="/billing" class="ml-1 underline font-medium">Upgrade</a>
          </div>
        {:else}
          {#if orgDetails.org.logo_url}
            <div class="flex items-center gap-4 mb-4">
              <img
                src={resolveLogoUrl(orgDetails.org.logo_url)}
                alt="Organization logo"
                class="w-16 h-16 object-contain rounded-lg border border-gray-200 bg-gray-50 p-1"
              />
              <div>
                <p class="text-sm font-medium text-gray-900">Logo uploaded</p>
                <p class="text-xs text-gray-500">
                  Displayed in QR codes when embed is enabled.
                </p>
              </div>
            </div>
          {/if}

          <div class="flex items-center gap-3">
            <label class="cursor-pointer">
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp,image/svg+xml"
                class="hidden"
                bind:this={logoFileInput}
                onchange={handleLogoUpload}
                disabled={logoUploading || !isOwner}
              />
              <span
                class="inline-flex items-center gap-2 px-4 py-2 bg-orange-500 hover:bg-orange-600 text-white rounded-lg text-sm font-medium transition-colors cursor-pointer {logoUploading ||
                !isOwner
                  ? 'opacity-50 cursor-not-allowed pointer-events-none'
                  : ''}"
              >
                {#if logoUploading}
                  <svg
                    class="animate-spin w-4 h-4"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      class="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      stroke-width="4"
                    ></circle>
                    <path
                      class="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    ></path>
                  </svg>
                  Uploading…
                {:else}
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
                      d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                    ></path>
                  </svg>
                  {orgDetails.org.logo_url ? "Replace Logo" : "Upload Logo"}
                {/if}
              </span>
            </label>

            {#if orgDetails.org.logo_url && isOwner}
              <button
                onclick={handleDeleteLogo}
                disabled={logoDeleting}
                class="px-4 py-2 border border-red-300 text-red-600 hover:bg-red-50 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
              >
                {logoDeleting ? "Removing…" : "Remove Logo"}
              </button>
            {/if}
          </div>

          {#if logoError}
            <p class="mt-2 text-sm text-red-600">{logoError}</p>
          {/if}
        {/if}
      </div>

      <!-- Members Card -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <h2 class="text-lg font-semibold text-gray-900 mb-4">
          Members
          <span class="text-sm font-normal text-gray-500 ml-1"
            >({orgDetails.members.length})</span
          >
        </h2>

        <ul class="divide-y divide-gray-100">
          {#each orgDetails.members as member (member.user_id)}
            <li class="flex items-center justify-between py-3">
              <div class="flex items-center gap-3">
                <Avatar user={member} size="sm" />
                <div>
                  <p class="text-sm font-medium text-gray-900">
                    {member.name ?? member.email}
                  </p>
                  {#if member.name}
                    <p class="text-xs text-gray-500">
                      {member.email}
                    </p>
                  {/if}
                </div>
              </div>
              <div class="flex items-center gap-3">
                <span
                  class="text-xs font-medium capitalize px-2 py-0.5 rounded-full {member.role ===
                  'owner'
                    ? 'bg-orange-100 text-orange-700'
                    : 'bg-gray-100 text-gray-600'}"
                >
                  {member.role}
                </span>
                {#if isOwner && member.user_id !== data.user?.id}
                  <button
                    onclick={() => handleRemoveMember(member)}
                    class="text-xs text-red-500 hover:text-red-700 transition-colors"
                    aria-label="Remove member"
                  >
                    Remove
                  </button>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      </div>

      <!-- Invite Card (owner only) -->
      {#if isOwner}
        <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
          <h2 class="text-lg font-semibold text-gray-900 mb-1">
            Invite Members
          </h2>

          {#if !canInviteMembers}
            <div
              class="bg-amber-50 border border-amber-200 rounded-lg p-3 mt-2 text-sm text-amber-800"
            >
              Inviting members requires the <strong>Business plan</strong>.
              Upgrade to collaborate with your team.
            </div>
          {:else}
            <p class="text-sm text-gray-500 mb-4">
              Send an invitation by email. The invitee will have 7 days to
              accept.
            </p>
            <div class="flex items-start gap-3">
              <div class="flex-1">
                <input
                  type="email"
                  bind:value={inviteEmail}
                  placeholder="colleague@example.com"
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
                  onkeydown={(e) => e.key === "Enter" && handleInvite()}
                />
                {#if inviteError}
                  <p class="mt-1 text-sm text-red-600">
                    {inviteError}
                  </p>
                {/if}
                {#if inviteSuccess}
                  <p class="mt-1 text-sm text-green-600">
                    {inviteSuccess}
                  </p>
                {/if}
              </div>
              <LoadingButton
                onclick={handleInvite}
                loading={inviting}
                variant="primary"
              >
                {inviting ? "Sending…" : "Send Invite"}
              </LoadingButton>
            </div>

            <!-- Pending Invitations -->
            {#if orgDetails.pending_invitations.length > 0}
              <div class="mt-5">
                <h3 class="text-sm font-medium text-gray-700 mb-2">
                  Pending Invitations
                </h3>
                <ul class="divide-y divide-gray-100">
                  {#each orgDetails.pending_invitations as inv (inv.id)}
                    <li class="flex items-center justify-between py-2.5">
                      <div>
                        <p class="text-sm text-gray-900">
                          {inv.email}
                        </p>
                        <p class="text-xs text-gray-500">
                          Expires {formatDate(inv.expires_at)}
                        </p>
                      </div>
                      <div class="flex items-center gap-3">
                        <button
                          onclick={() => handleResendInvitation(inv)}
                          class="text-xs text-orange-500 hover:text-orange-700 transition-colors"
                        >
                          Resend
                        </button>
                        <button
                          onclick={() => handleRevokeInvitation(inv)}
                          class="text-xs text-red-500 hover:text-red-700 transition-colors"
                        >
                          Revoke
                        </button>
                      </div>
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}
          {/if}
        </div>
      {/if}

      <!-- Plan & Billing -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <h2 class="text-lg font-semibold text-gray-900 mb-1">
          Plan &amp; Billing
        </h2>
        <p class="text-xs text-gray-500 capitalize mb-4">
          Current plan: <span class="font-medium text-gray-700"
            >{orgDetails.org.tier}</span
          >
        </p>
        {#if billingStatus?.is_billing_owner}
          <p class="text-sm text-gray-600 mb-3">
            You own the billing account for this organization.
          </p>
          <a
            href="/billing"
            class="inline-flex items-center gap-2 px-4 py-2 bg-orange-500 hover:bg-orange-600 text-white rounded-lg text-sm font-medium transition-colors"
          >
            Manage Billing &amp; Subscription
          </a>
        {:else}
          <p class="text-sm text-gray-500">
            Billing is managed by the owner of this billing account. Contact
            them to change the plan.
          </p>
        {/if}
      </div>

      <!-- Tags Management -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <h2 class="text-lg font-semibold text-gray-900 mb-4">Tags</h2>

        {#if tagsLoading}
          <div class="flex items-center gap-2 text-sm text-gray-500">
            <div
              class="animate-spin w-4 h-4 border-2 border-gray-300 border-t-gray-600 rounded-full"
            ></div>
            Loading tags...
          </div>
        {:else if tagsError}
          <div class="text-sm text-red-600">
            {tagsError}
          </div>
        {:else if tags.length === 0}
          <p class="text-sm text-gray-500">
            No tags created yet. Tags are automatically created when you add
            them to links.
          </p>
        {:else}
          <div class="space-y-2">
            {#each tags as tag (tag.name)}
              <div
                class="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
              >
                <div class="flex items-center gap-3">
                  <span
                    class="inline-block w-3 h-3 rounded-full {tagColor(
                      tag.name
                    ).split(' ')[0]}"
                  ></span>
                  {#if editingTag === tag.name}
                    <input
                      type="text"
                      bind:value={newTagName}
                      onkeydown={(e) => {
                        if (e.key === "Enter") saveTagRename(tag.name);
                        if (e.key === "Escape") cancelEditTag();
                      }}
                      class="px-2 py-1 text-sm border border-gray-300 rounded focus:border-orange-500 focus:outline-none"
                      maxlength="50"
                    />
                  {:else}
                    <span class="font-medium text-gray-900">{tag.name}</span>
                  {/if}
                  <span class="text-sm text-gray-500">({tag.count} links)</span>
                </div>

                <div class="flex items-center gap-2">
                  {#if editingTag === tag.name}
                    <button
                      onclick={() => saveTagRename(tag.name)}
                      disabled={savingTag}
                      class="text-xs px-2 py-1 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {savingTag ? "Saving..." : "Save"}
                    </button>
                    <button
                      onclick={cancelEditTag}
                      disabled={savingTag}
                      class="text-xs px-2 py-1 bg-gray-600 text-white rounded hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Cancel
                    </button>
                  {:else}
                    <button
                      onclick={() => startEditTag(tag.name)}
                      class="text-xs px-2 py-1 text-blue-600 hover:text-blue-700"
                    >
                      Rename
                    </button>
                    <button
                      onclick={() => startDeleteTag(tag.name)}
                      class="text-xs px-2 py-1 text-red-600 hover:text-red-700"
                    >
                      Delete
                    </button>
                  {/if}
                </div>
              </div>

              {#if editingTag === tag.name && tagError}
                <div class="ml-3 text-xs text-red-600">
                  {tagError}
                </div>
              {/if}

              {#if deletingTag === tag.name}
                <div
                  class="ml-3 p-3 bg-red-50 border border-red-200 rounded-lg"
                >
                  <p class="text-sm text-red-800 mb-3">
                    Are you sure you want to delete the tag "{tag.name}"? This
                    will remove it from {tag.count}
                    link{tag.count === 1 ? "" : "s"}.
                  </p>
                  <div class="flex gap-2">
                    <button
                      onclick={() => confirmDeleteTag(tag.name)}
                      class="text-xs px-3 py-1 bg-red-600 text-white rounded hover:bg-red-700"
                    >
                      Delete
                    </button>
                    <button
                      onclick={cancelDeleteTag}
                      class="text-xs px-3 py-1 bg-gray-600 text-white rounded hover:bg-gray-700"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              {/if}
            {/each}
          </div>
        {/if}
      </div>

      <!-- Danger Zone (owner only, multiple orgs) -->
      {#if isOwner}
        <div class="bg-white rounded-xl border border-red-200 p-6">
          <h2 class="text-lg font-semibold text-red-700 mb-2">Danger Zone</h2>
          <p class="text-sm text-gray-600 mb-4">
            Deleting an organization is permanent and cannot be undone.
          </p>
          <button
            onclick={openDeleteModal}
            class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm font-medium transition-colors"
          >
            Delete Organization
          </button>
        </div>
      {/if}
    {/if}
  </main>
</div>

<!-- Delete Organization Modal -->
{#if showDeleteModal && orgDetails}
  <div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
  >
    <div class="bg-white rounded-xl max-w-lg w-full p-6 shadow-2xl">
      <h2 class="text-xl font-bold text-gray-900 mb-4">Delete Organization</h2>

      <div
        class="mb-4 bg-red-50 border border-red-200 rounded-lg p-3 text-sm text-red-800"
      >
        <strong>Warning:</strong> This action cannot be undone. You are about to
        delete <strong>{orgDetails.org.name}</strong>.
      </div>

      <p class="text-sm text-gray-700 mb-4">
        What would you like to do with the <strong
          >{linkCount} link{linkCount === 1 ? "" : "s"}</strong
        > in this organization?
      </p>

      <div class="space-y-3 mb-6">
        <!-- Delete All Option -->
        <label
          class="flex items-start gap-3 p-3 border-2 rounded-lg cursor-pointer transition-colors {deleteAction ===
          'delete'
            ? 'border-orange-500 bg-orange-50'
            : 'border-gray-200 hover:border-gray-300'}"
        >
          <input
            type="radio"
            name="deleteAction"
            value="delete"
            bind:group={deleteAction}
            class="mt-0.5"
          />
          <div class="flex-1">
            <div class="font-medium text-gray-900">Delete all links</div>
            <div class="text-sm text-gray-600 mt-1">
              All links and analytics data will be permanently removed.
            </div>
          </div>
        </label>

        <!-- Migrate Option -->
        {#if userOrgs.length > 0}
          <label
            class="flex items-start gap-3 p-3 border-2 rounded-lg cursor-pointer transition-colors {deleteAction ===
            'migrate'
              ? 'border-orange-500 bg-orange-50'
              : 'border-gray-200 hover:border-gray-300'}"
          >
            <input
              type="radio"
              name="deleteAction"
              value="migrate"
              bind:group={deleteAction}
              class="mt-0.5"
            />
            <div class="flex-1">
              <div class="font-medium text-gray-900">
                Migrate links to another organization
              </div>
              <div class="text-sm text-gray-600 mt-1 mb-2">
                Transfer all links to one of your other organizations.
              </div>
              {#if deleteAction === "migrate"}
                <select
                  bind:value={targetOrgId}
                  class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
                >
                  {#each userOrgs as org (org.id)}
                    <option value={org.id}>{org.name} ({org.tier} plan)</option>
                  {/each}
                </select>
                <p class="text-xs text-gray-500 mt-1">
                  Make sure the target organization has enough available slots.
                </p>
              {/if}
            </div>
          </label>
        {:else}
          <div
            class="p-3 bg-gray-50 border border-gray-200 rounded-lg text-sm text-gray-600"
          >
            You don't have other organizations to migrate links to.
          </div>
        {/if}
      </div>

      {#if deleteError}
        <div
          class="mb-4 bg-red-50 border border-red-200 rounded-lg p-3 text-sm text-red-700"
        >
          {deleteError}
        </div>
      {/if}

      <div class="flex gap-3 justify-end">
        <button
          onclick={() => {
            showDeleteModal = false;
            deleteError = "";
          }}
          disabled={deleting}
          class="px-4 py-2 text-gray-700 hover:text-gray-900 border border-gray-300 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
        >
          Cancel
        </button>
        <LoadingButton
          onclick={handleDeleteOrg}
          loading={deleting}
          disabled={deleteAction === "migrate" && !targetOrgId}
          variant="danger"
        >
          {deleting ? "Deleting…" : "Delete Organization"}
        </LoadingButton>
      </div>
    </div>
  </div>
{/if}

<!-- Remove Member Confirmation Modal -->
{#if confirmingRemoveMember}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={cancelRemoveMember}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        cancelRemoveMember();
      }
    }}
  >
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
      <div class="modal-header">
        <h3>Remove Member?</h3>
        <button class="modal-close" onclick={cancelRemoveMember}>&times;</button
        >
      </div>
      <div class="modal-body">
        <p>
          Are you sure you want to <strong
            >remove {confirmingRemoveMember.email}</strong
          >
          from this organization? They will lose access to all links and resources.
        </p>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={cancelRemoveMember}>
          Cancel
        </button>
        <button class="btn btn-danger" onclick={confirmRemoveMember}>
          Remove Member
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
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

  .btn-secondary {
    background: #64748b;
    color: white;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #475569;
  }

  .btn-danger {
    background: #dc2626;
    color: white;
  }

  .btn-danger:hover:not(:disabled) {
    background: #b91c1c;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
