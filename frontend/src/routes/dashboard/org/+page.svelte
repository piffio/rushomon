<script lang="ts">
  import type { BillingStatus } from "$lib/api/billing";
  import { billingApi } from "$lib/api/billing";
  import { resolveLogoUrl } from "$lib/api/client";
  import type { CreateDomainResponse, CustomDomain } from "$lib/api/domains";
  import { domainsApi } from "$lib/api/domains";
  import { tagsApi } from "$lib/api/links";
  import { orgsApi } from "$lib/api/orgs";
  import { usageApi } from "$lib/api/usage";
  import Avatar from "$lib/components/Avatar.svelte";
  import LoadingButton from "$lib/components/LoadingButton.svelte";
  import type {
    OrgDetails,
    OrgInvitation,
    OrgMember,
    OrgSettings,
    OrgWithRole,
    TagAnalytics,
    TagWithCount,
    UsageResponse
  } from "$lib/types/api";
  import type { PageData } from "./$types";

  const { data }: { data: PageData } = $props();

  let orgDetails = $state<OrgDetails | null>(null);
  let loading = $state(true);
  let error = $state("");
  let billingStatus = $state<BillingStatus | null>(null);
  let usage = $state<UsageResponse | null>(null);

  // Rename org
  let editingName = $state(false);
  let newOrgName = $state("");
  let savingName = $state(false);
  let nameError = $state("");

  // Invite member
  let inviteEmail = $state("");
  let inviteRole = $state<"member" | "admin">("member");
  let inviting = $state(false);
  let inviteError = $state("");
  let inviteSuccess = $state("");

  // General feedback
  let actionError = $state("");
  let actionSuccess = $state("");
  let actionWarning = $state("");

  // Custom domains
  let domains = $state<CustomDomain[]>([]);
  let domainsLoading = $state(false);
  let domainsError = $state("");
  let newHostname = $state("");
  let addingDomain = $state(false);
  let addDomainError = $state("");
  let newDomainResult = $state<CreateDomainResponse | null>(null);
  let refreshingDomain = $state<string | null>(null);
  let deletingDomain = $state<string | null>(null);

  // Copy to clipboard feedback
  let copiedValue = $state<string | null>(null);
  let copyTimeout: ReturnType<typeof setTimeout> | null = null;

  async function copyToClipboard(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      copiedValue = value;
      if (copyTimeout) clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => {
        copiedValue = null;
      }, 2000);
    } catch {
      // Fallback for browsers that don't support clipboard API
      console.error("Failed to copy to clipboard");
    }
  }
  let confirmingDomainHostname = $state<string | null>(null);

  // Tags management
  let tags = $state<TagWithCount[]>([]);
  let tagsLoading = $state(false);
  let tagsError = $state("");
  let deletingTag = $state<string | null>(null);

  // Enhanced tag management
  let selectedTags = $state(new Set<string>());
  let tagSearchQuery = $state("");
  let tagSortField = $state<"name" | "count" | "created_at" | "last_used_at">(
    "count"
  );
  let tagSortDirection = $state<"asc" | "desc">("desc");
  let isMergeModalOpen = $state(false);
  let mergeDestinationTag = $state("");
  let mergingTags = $state(false);
  let mergeError = $state("");
  let isCreateTagModalOpen = $state(false);
  let newTagNameInput = $state("");
  let selectedColorIndex = $state<number | null>(null);
  let creatingTag = $state(false);
  let createTagError = $state("");
  let isEditTagModalOpen = $state(false);
  let editingTagName = $state("");
  let editTagNewName = $state("");
  let editTagColorIndex = $state<number | null>(null);
  let savingEditTag = $state(false);
  let editTagError = $state("");
  let showAnalytics = $state(false);
  let tagAnalytics = $state<TagAnalytics | null>(null);
  let analyticsLoading = $state(false);

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
      await loadDomains(currentOrgId);
      usage = await usageApi.getUsage();
    } catch (e: unknown) {
      error =
        e instanceof Error ? e.message : "Failed to load organization details.";
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadOrg();
    checkCanDelete();
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
      await orgsApi.inviteMember(orgDetails.org.id, email, inviteRole);
      inviteSuccess = `Invitation sent to ${email} as ${inviteRole}.`;
      inviteEmail = "";
      inviteRole = "member";
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

  async function handleUpdateMemberRole(
    member: OrgMember,
    newRole: "member" | "admin"
  ) {
    if (!orgDetails || newRole === member.role) return;
    updatingRoleMemberId = member.user_id;
    actionError = "";
    actionSuccess = "";
    try {
      await orgsApi.updateMemberRole(
        orgDetails.org.id,
        member.user_id,
        newRole
      );
      actionSuccess = `${member.name ?? member.email}'s role updated to ${newRole}.`;
      await loadOrg();
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      actionError = e instanceof Error ? e.message : "Failed to update role.";
    } finally {
      updatingRoleMemberId = null;
    }
  }

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric"
    });
  }

  const isOwner = $derived(orgDetails?.org.role === "owner");
  const isAdmin = $derived(orgDetails?.org.role === "admin");
  const canManageMembers = $derived(isOwner || isAdmin);
  const isPro = $derived(
    ["pro", "business", "unlimited"].includes(orgDetails?.org.tier ?? "")
  );
  // Business tier only allows team members (up to 20)
  const canInviteMembers = $derived(
    ["business", "unlimited"].includes(orgDetails?.org.tier ?? "")
  );

  // Custom domain quota check
  const maxCustomDomains = $derived(usage?.limits.max_custom_domains);
  const activeDomainCount = $derived(
    domains.filter((d) => d.status === "active").length
  );
  const domainQuotaReached = $derived(
    usage !== null &&
      maxCustomDomains !== null &&
      maxCustomDomains !== undefined &&
      maxCustomDomains > 0 &&
      activeDomainCount >= maxCustomDomains
  );
  const domainFeatureLocked = $derived(
    usage !== null && maxCustomDomains === 0
  );
  const domainUpgradeTarget = $derived(() => {
    const tier = orgDetails?.org.tier ?? "free";
    if (tier === "free") return "Pro";
    if (tier === "pro") return "Business";
    return null;
  });

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

  async function loadDomains(orgId: string) {
    domainsLoading = true;
    domainsError = "";
    try {
      domains = await domainsApi.listDomains(orgId);
      // Check if any domain has pending SSL status and fetch DNS instructions
      const pendingSslDomain = domains.find((d) => d.ssl_status === "pending");
      if (pendingSslDomain) {
        try {
          const result = await domainsApi.refreshDomain(
            orgId,
            pendingSslDomain.hostname
          );
          if (result.dns_instructions?.needs_txt) {
            newDomainResult = result;
          } else {
            // Clear DNS instructions if SSL is now active
            if (
              newDomainResult?.domain.hostname === pendingSslDomain.hostname
            ) {
              newDomainResult = null;
            }
          }
        } catch {
          // Non-critical - if refresh fails, we'll just not show DNS instructions
        }
      } else {
        // If no domains have pending SSL, clear any stale DNS instructions
        newDomainResult = null;
      }
    } catch (e: unknown) {
      domainsError = e instanceof Error ? e.message : "Failed to load domains.";
    } finally {
      domainsLoading = false;
    }
  }

  async function handleAddDomain() {
    if (!orgDetails || !newHostname.trim()) return;
    addingDomain = true;
    addDomainError = "";
    newDomainResult = null;
    try {
      const result = await domainsApi.addDomain(
        orgDetails.org.id,
        newHostname.trim()
      );
      domains = [...domains, result.domain];
      newDomainResult = result;
      newHostname = "";
    } catch (e: unknown) {
      addDomainError = e instanceof Error ? e.message : "Failed to add domain.";
    } finally {
      addingDomain = false;
    }
  }

  async function handleDeleteDomain(hostname: string) {
    if (!orgDetails) return;
    confirmingDomainHostname = hostname;
  }

  async function confirmDeleteDomain() {
    if (!orgDetails || !confirmingDomainHostname) return;
    const hostname = confirmingDomainHostname;
    deletingDomain = hostname;
    try {
      const result = await domainsApi.deleteDomain(orgDetails.org.id, hostname);
      domains = domains.filter((d) => d.hostname !== hostname);
      if (newDomainResult?.domain.hostname === hostname) newDomainResult = null;

      // Show warning if domain was not found in Cloudflare
      if (!result.cf_deleted && result.cf_deleted_message) {
        actionWarning = result.cf_deleted_message;
        setTimeout(() => (actionWarning = ""), 10000);
      }
    } catch (e: unknown) {
      actionError = e instanceof Error ? e.message : "Failed to remove domain.";
      setTimeout(() => (actionError = ""), 5000);
    } finally {
      deletingDomain = null;
      confirmingDomainHostname = null;
    }
  }

  function closeConfirmDomain() {
    confirmingDomainHostname = null;
  }

  async function handleRefreshDomain(hostname: string) {
    if (!orgDetails) return;
    refreshingDomain = hostname;
    try {
      const result = await domainsApi.refreshDomain(
        orgDetails.org.id,
        hostname
      );
      domains = domains.map((d) =>
        d.hostname === hostname ? result.domain : d
      );
      // If refresh returned SSL validation records and domain is still pending,
      // show them in the DNS instructions panel
      if (result.dns_instructions?.needs_txt) {
        newDomainResult = result;
      }
    } catch (e: unknown) {
      actionError =
        e instanceof Error ? e.message : "Failed to refresh domain status.";
      setTimeout(() => (actionError = ""), 5000);
    } finally {
      refreshingDomain = null;
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
  let updatingRoleMemberId = $state<string | null>(null);
  let linkCount = $state(0);
  let userOrgs = $state<OrgWithRole[]>([]);
  let canDelete = $state(false);

  async function checkCanDelete() {
    try {
      const orgsRes = await orgsApi.listMyOrgs();
      const ownedOrgs = orgsRes.orgs.filter((o) => o.role === "owner");
      userOrgs = ownedOrgs.filter((o) => o.id !== orgDetails!.org.id);
      canDelete = ownedOrgs.length > 1;

      // Get link count from org details (actual links in this org, not billing account level)
      linkCount = orgDetails?.org.link_count || 0;
    } catch {
      canDelete = false;
    }
  }

  async function openDeleteModal() {
    if (!canDelete) {
      return;
    }

    // If org has no links, delete directly without showing modal
    if (linkCount === 0) {
      deleteAction = "delete";
      await handleDeleteOrg();
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
      selectedTags.delete(tagName);
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

  // Enhanced tag management functions
  function toggleTagSelection(tagName: string) {
    if (selectedTags.has(tagName)) {
      selectedTags.delete(tagName);
    } else {
      selectedTags.add(tagName);
    }
  }

  function selectAllTags() {
    filteredTags().forEach((t) => selectedTags.add(t.name));
  }

  function deselectAllTags() {
    filteredTags().forEach((t) => selectedTags.delete(t.name));
  }

  function setTagSort(field: "name" | "count" | "created_at" | "last_used_at") {
    if (tagSortField === field) {
      tagSortDirection = tagSortDirection === "asc" ? "desc" : "asc";
    } else {
      tagSortField = field;
      tagSortDirection = "desc";
    }
  }

  // Derived state for filtered and sorted tags
  const filteredTags = $derived(() => {
    let result = tags;

    // Filter by search query
    if (tagSearchQuery.trim()) {
      const query = tagSearchQuery.toLowerCase();
      result = result.filter((t) => t.name.toLowerCase().includes(query));
    }

    // Sort
    result = [...result].sort((a, b) => {
      let comparison = 0;
      switch (tagSortField) {
        case "name":
          comparison = a.name.localeCompare(b.name);
          break;
        case "count":
          comparison = a.count - b.count;
          break;
        case "created_at":
          comparison = a.created_at - b.created_at;
          break;
        case "last_used_at": {
          const aLastUsed = a.last_used_at ?? 0;
          const bLastUsed = b.last_used_at ?? 0;
          comparison = aLastUsed - bLastUsed;
          break;
        }
      }
      return tagSortDirection === "asc" ? comparison : -comparison;
    });

    return result;
  });

  // Bulk delete unused selected tags
  async function bulkDeleteUnusedTags() {
    const unusedSelected = Array.from(selectedTags).filter((name) => {
      const tag = tags.find((t) => t.name === name);
      return tag && tag.count === 0;
    }) as string[];

    if (unusedSelected.length === 0) return;

    try {
      for (const tagName of unusedSelected) {
        await tagsApi.remove(tagName);
      }
      tags = tags.filter((t) => !unusedSelected.includes(t.name));
      unusedSelected.forEach((name) => selectedTags.delete(name));
      actionSuccess = `${unusedSelected.length} unused tag${unusedSelected.length === 1 ? "" : "s"} deleted.`;
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      actionError = e instanceof Error ? e.message : "Failed to delete tags.";
      setTimeout(() => (actionError = ""), 3000);
    }
  }

  // Merge functions
  function openMergeModal() {
    if (selectedTags.size < 2) {
      actionError = "Select at least 2 tags to merge.";
      setTimeout(() => (actionError = ""), 3000);
      return;
    }
    mergeDestinationTag = "";
    mergeError = "";
    isMergeModalOpen = true;
  }

  function closeMergeModal() {
    isMergeModalOpen = false;
    mergeDestinationTag = "";
    mergeError = "";
  }

  async function confirmMergeTags() {
    if (!mergeDestinationTag.trim()) {
      mergeError = "Please select a destination tag.";
      return;
    }

    const sourceTags = Array.from(selectedTags).filter(
      (t) => t !== mergeDestinationTag
    ) as string[];
    if (sourceTags.length === 0) {
      mergeError = "Source tags cannot include the destination tag.";
      return;
    }

    mergingTags = true;
    mergeError = "";
    try {
      const result = await tagsApi.merge({
        source_tags: sourceTags,
        destination_tag: mergeDestinationTag
      });
      tags = await tagsApi.list();
      selectedTags.clear();
      closeMergeModal();
      actionSuccess = `Merged ${result.merged_tags.length} tags into "${result.destination_tag}" (${result.affected_links} links affected).`;
      setTimeout(() => (actionSuccess = ""), 5000);
    } catch (e: unknown) {
      mergeError = e instanceof Error ? e.message : "Failed to merge tags.";
    } finally {
      mergingTags = false;
    }
  }

  // Create tag functions
  function openCreateTagModal() {
    newTagNameInput = "";
    selectedColorIndex = null;
    createTagError = "";
    isCreateTagModalOpen = true;
  }

  function closeCreateTagModal() {
    isCreateTagModalOpen = false;
    newTagNameInput = "";
    selectedColorIndex = null;
    createTagError = "";
  }

  // Edit tag functions
  function openEditTagModal(tag: TagWithCount) {
    editingTagName = tag.name;
    editTagNewName = tag.name;
    editTagColorIndex = tag.color_index;
    editTagError = "";
    isEditTagModalOpen = true;
  }

  function closeEditTagModal() {
    isEditTagModalOpen = false;
    editingTagName = "";
    editTagNewName = "";
    editTagColorIndex = null;
    editTagError = "";
  }

  async function confirmEditTag() {
    const trimmed = editTagNewName.trim();
    if (!trimmed) {
      editTagError = "Tag name cannot be empty.";
      return;
    }
    if (trimmed.length > 50) {
      editTagError = "Tag name must be 50 characters or less.";
      return;
    }

    savingEditTag = true;
    editTagError = "";
    try {
      const updateData: { new_name?: string; color_index?: number } = {};

      // Only include new_name if it changed
      if (trimmed !== editingTagName) {
        updateData.new_name = trimmed;
      }

      // Only include color_index if it changed
      const currentTag = tags.find((t) => t.name === editingTagName);
      if (editTagColorIndex !== currentTag?.color_index) {
        updateData.color_index = editTagColorIndex ?? 0;
      }

      // Check if anything changed
      if (Object.keys(updateData).length === 0) {
        closeEditTagModal();
        return;
      }

      tags = await tagsApi.update(editingTagName, updateData);
      closeEditTagModal();
      actionSuccess = `Tag "${editingTagName}" updated successfully.`;
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      editTagError = e instanceof Error ? e.message : "Failed to update tag.";
    } finally {
      savingEditTag = false;
    }
  }

  async function confirmCreateTag() {
    const trimmed = newTagNameInput.trim();
    if (!trimmed) {
      createTagError = "Tag name cannot be empty.";
      return;
    }
    if (trimmed.length > 50) {
      createTagError = "Tag name must be 50 characters or less.";
      return;
    }
    if (tags.some((t) => t.name.toLowerCase() === trimmed.toLowerCase())) {
      createTagError = "A tag with this name already exists.";
      return;
    }

    creatingTag = true;
    createTagError = "";
    try {
      tags = await tagsApi.create(trimmed, selectedColorIndex ?? undefined);
      closeCreateTagModal();
      actionSuccess = `Tag "${trimmed}" created successfully.`;
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      createTagError = e instanceof Error ? e.message : "Failed to create tag.";
    } finally {
      creatingTag = false;
    }
  }

  // Analytics functions
  async function loadTagAnalytics() {
    analyticsLoading = true;
    try {
      tagAnalytics = await tagsApi.getAnalytics();
    } catch (e: unknown) {
      console.error("Failed to load analytics:", e);
    } finally {
      analyticsLoading = false;
    }
  }

  function toggleAnalytics() {
    showAnalytics = !showAnalytics;
    if (showAnalytics && !tagAnalytics) {
      loadTagAnalytics();
    }
  }

  // Quick merge from analytics suggestion
  async function quickMergeTags(sourceTags: string[], destinationTag: string) {
    try {
      const result = await tagsApi.merge({
        source_tags: sourceTags.filter((t) => t !== destinationTag),
        destination_tag: destinationTag
      });
      tags = await tagsApi.list();
      if (tagAnalytics) {
        await loadTagAnalytics();
      }
      actionSuccess = `Merged ${result.merged_tags.length} tags into "${result.destination_tag}".`;
      setTimeout(() => (actionSuccess = ""), 3000);
    } catch (e: unknown) {
      actionError = e instanceof Error ? e.message : "Failed to merge tags.";
      setTimeout(() => (actionError = ""), 3000);
    }
  }

  // Helper function for tag colors (same as SearchFilterBar)
  // 8 visually distinct colors across the spectrum
  const TAG_COLORS = [
    "bg-red-100 text-red-800",
    "bg-orange-100 text-orange-800",
    "bg-yellow-100 text-yellow-800",
    "bg-green-100 text-green-800",
    "bg-cyan-100 text-cyan-800",
    "bg-blue-100 text-blue-800",
    "bg-violet-100 text-violet-800",
    "bg-pink-100 text-pink-800"
  ];

  function tagColor(tag: string, colorIndex?: number | null): string {
    // Use stored color index if available, otherwise fall back to hash-based
    if (colorIndex !== undefined && colorIndex !== null) {
      return TAG_COLORS[colorIndex % TAG_COLORS.length];
    }
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
      {#if actionWarning}
        <div
          class="mb-4 bg-amber-50 border border-amber-200 rounded-xl p-3 text-amber-700 text-sm"
        >
          {actionWarning}
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
                <!-- Role badge / selector -->
                {#if canManageMembers && member.role !== "owner" && member.user_id !== data.user?.id && !(isAdmin && member.role === "admin")}
                  <select
                    value={member.role}
                    disabled={updatingRoleMemberId === member.user_id}
                    onchange={(e) =>
                      handleUpdateMemberRole(
                        member,
                        (e.currentTarget as HTMLSelectElement).value as
                          | "member"
                          | "admin"
                      )}
                    class="text-xs font-medium px-2 py-0.5 rounded-full border border-gray-200 bg-white text-gray-700 cursor-pointer disabled:opacity-50"
                    aria-label="Change role"
                  >
                    <option value="member">Member</option>
                    <option value="admin">Admin</option>
                  </select>
                {:else}
                  <span
                    class="text-xs font-medium capitalize px-2 py-0.5 rounded-full {member.role ===
                    'owner'
                      ? 'bg-orange-100 text-orange-700'
                      : member.role === 'admin'
                        ? 'bg-blue-100 text-blue-700'
                        : 'bg-gray-100 text-gray-600'}"
                  >
                    {member.role}
                  </span>
                {/if}
                <!-- Remove button: owner can remove any non-self; admin can remove members (not owners/admins) -->
                {#if member.user_id !== data.user?.id && (isOwner || (isAdmin && member.role === "member"))}
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

      <!-- Invite Card (owner or admin) -->
      {#if canManageMembers}
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
              <select
                bind:value={inviteRole}
                class="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none bg-white"
                aria-label="Role for invitee"
              >
                <option value="member">Member</option>
                <option value="admin">Admin</option>
              </select>
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
                          <span
                            class="ml-1 text-xs font-medium capitalize px-1.5 py-0.5 rounded-full {inv.role ===
                            'admin'
                              ? 'bg-blue-100 text-blue-700'
                              : 'bg-gray-100 text-gray-600'}">{inv.role}</span
                          >
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
        {#if billingStatus?.billing_account_id && orgDetails?.org.billing_account_id && billingStatus.billing_account_id === orgDetails.org.billing_account_id}
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

      <!-- Custom Domains -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <div class="flex items-center justify-between mb-1">
          <h2 class="text-lg font-semibold text-gray-900">Custom Domains</h2>
          {#if orgDetails.org.tier === "free"}
            <span
              class="text-xs bg-amber-100 text-amber-700 px-2 py-1 rounded-full font-medium"
              >Pro+ feature</span
            >
          {/if}
        </div>
        <p class="text-sm text-gray-500 mb-4">
          Point your own domain at your short links (e.g. <code
            class="bg-gray-100 px-1 rounded">go.example.com</code
          >).
        </p>

        {#if orgDetails.org.tier === "free"}
          <div class="bg-orange-50 border border-orange-200 rounded-lg p-4">
            <p class="text-sm text-orange-800">
              Custom domains require a <strong>Pro plan</strong> or higher.
              <a href="/billing" class="underline font-medium"
                >Upgrade your plan</a
              > to add custom domains.
            </p>
          </div>
        {:else}
          {#if domainsLoading}
            <div class="flex items-center gap-2 text-sm text-gray-500">
              <div
                class="animate-spin w-4 h-4 border-2 border-gray-300 border-t-gray-600 rounded-full"
              ></div>
              Loading domains...
            </div>
          {:else if domainsError}
            <div class="text-sm text-red-600 mb-3">{domainsError}</div>
          {/if}

          {#if domains.length > 0}
            <ul class="space-y-3 mb-4">
              {#each domains as domain (domain.id)}
                <li
                  class="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                >
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                      <span class="font-mono text-sm text-gray-800 truncate"
                        >{domain.hostname}</span
                      >
                      {#if domain.status === "active" && domain.ssl_status === "pending"}
                        <span
                          class="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded-full"
                          >Pending Certificate</span
                        >
                      {:else if domain.status === "active"}
                        <span
                          class="text-xs bg-green-100 text-green-700 px-2 py-0.5 rounded-full"
                          >Active</span
                        >
                      {:else if domain.status === "pending"}
                        <span
                          class="text-xs bg-yellow-100 text-yellow-700 px-2 py-0.5 rounded-full"
                          >Pending DNS</span
                        >
                      {:else if domain.status === "inactive_downgrade"}
                        <span
                          class="text-xs bg-gray-100 text-gray-600 px-2 py-0.5 rounded-full"
                          >Inactive – Upgrade Required</span
                        >
                      {:else}
                        <span
                          class="text-xs bg-red-100 text-red-700 px-2 py-0.5 rounded-full"
                          >Failed</span
                        >
                      {/if}
                    </div>
                    {#if domain.status === "pending"}
                      <p class="text-xs text-gray-500 mt-1">
                        Add the CNAME record, then click Refresh to check
                        status.
                      </p>
                    {:else if domain.status === "inactive_downgrade"}
                      <p class="text-xs text-gray-500 mt-1">
                        This domain was deactivated due to a plan downgrade.
                        <a href="/billing" class="text-blue-600 hover:underline"
                          >Upgrade your plan</a
                        >
                        to reactivate it and create new links.
                      </p>
                    {/if}
                  </div>
                  <div class="flex items-center gap-2 ml-4">
                    {#if domain.status === "pending" || domain.status === "active"}
                      <button
                        onclick={() => handleRefreshDomain(domain.hostname)}
                        disabled={refreshingDomain === domain.hostname}
                        class="text-xs text-blue-600 hover:text-blue-800 disabled:opacity-50 transition-colors"
                      >
                        {refreshingDomain === domain.hostname
                          ? "Checking..."
                          : "Refresh"}
                      </button>
                    {/if}
                    <button
                      onclick={() => handleDeleteDomain(domain.hostname)}
                      disabled={deletingDomain === domain.hostname}
                      class="text-xs text-red-500 hover:text-red-700 disabled:opacity-50 transition-colors"
                    >
                      {deletingDomain === domain.hostname
                        ? "Removing..."
                        : "Remove"}
                    </button>
                  </div>
                </li>
              {/each}
            </ul>
          {:else if !domainsLoading}
            <p class="text-sm text-gray-500 mb-4">
              No custom domains configured yet.
            </p>
          {/if}

          <!-- Add domain form -->
          {#if newDomainResult?.dns_instructions}
            <div class="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-4">
              <h3 class="text-sm font-semibold text-blue-900 mb-2">
                DNS Setup Required
              </h3>
              <p class="text-sm text-blue-800 mb-3">
                Add the following DNS records at your DNS provider to verify
                ownership and enable routing:
              </p>
              <div class="space-y-2">
                {#if newDomainResult?.dns_instructions?.needs_cname}
                  <div class="bg-white rounded border border-blue-200 p-4">
                    <div class="text-xs font-medium text-gray-500 mb-3">
                      CNAME Record
                    </div>
                    <div class="space-y-2">
                      <div>
                        <div class="text-xs text-gray-400 mb-1">Name</div>
                        <div class="flex items-center gap-2">
                          <span
                            class="font-mono text-sm text-gray-700 bg-gray-50 px-2 py-1 rounded"
                            >{newDomainResult.domain.hostname}</span
                          >
                          <button
                            onclick={() =>
                              copyToClipboard(newDomainResult!.domain.hostname)}
                            class="text-xs text-blue-600 hover:text-blue-800 underline"
                            title="Copy"
                          >
                            {copiedValue === newDomainResult!.domain.hostname
                              ? "Copied!"
                              : "Copy"}
                          </button>
                        </div>
                      </div>
                      <div>
                        <div class="text-xs text-gray-400 mb-1">Target</div>
                        <div class="flex items-center gap-2">
                          <span
                            class="font-mono text-sm text-blue-700 bg-blue-50 px-2 py-1 rounded"
                            >{newDomainResult.dns_instructions
                              .cname_target}</span
                          >
                          <button
                            onclick={() =>
                              copyToClipboard(
                                newDomainResult!.dns_instructions!.cname_target
                              )}
                            class="text-xs text-blue-600 hover:text-blue-800 underline"
                            title="Copy"
                          >
                            {copiedValue ===
                            newDomainResult!.dns_instructions!.cname_target
                              ? "Copied!"
                              : "Copy"}
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                {/if}
                {#if newDomainResult?.dns_instructions?.needs_txt && newDomainResult.dns_instructions.txt_records.length > 0}
                  {#each newDomainResult.dns_instructions.txt_records as record, index (index)}
                    <div
                      class="bg-white rounded border border-blue-200 p-4 mb-2 last:mb-0"
                    >
                      <div class="text-xs font-medium text-gray-500 mb-3">
                        TXT Record
                        {record.purpose === "ownership"
                          ? " (domain ownership verification)"
                          : " (SSL certificate validation)"}
                      </div>
                      <div class="space-y-2">
                        <div>
                          <div class="text-xs text-gray-400 mb-1">Name</div>
                          <div class="flex items-center gap-2">
                            <span
                              class="font-mono text-sm text-gray-700 bg-gray-50 px-2 py-1 rounded"
                              >{record.name}</span
                            >
                            <button
                              onclick={() => copyToClipboard(record.name)}
                              class="text-xs text-blue-600 hover:text-blue-800 underline"
                              title="Copy"
                            >
                              {copiedValue === record.name ? "Copied!" : "Copy"}
                            </button>
                          </div>
                        </div>
                        <div>
                          <div class="text-xs text-gray-400 mb-1">Value</div>
                          <div class="flex items-start gap-2">
                            <span
                              class="font-mono text-sm text-blue-700 bg-blue-50 px-2 py-1 rounded break-all flex-1"
                              >{record.value}</span
                            >
                            <button
                              onclick={() => copyToClipboard(record.value)}
                              class="text-xs text-blue-600 hover:text-blue-800 underline"
                              title="Copy"
                            >
                              {copiedValue === record.value
                                ? "Copied!"
                                : "Copy"}
                            </button>
                          </div>
                        </div>
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
              <p class="text-xs text-blue-700 mt-3">
                DNS propagation can take a few minutes. Use the Refresh button
                to check verification status.
              </p>
              <button
                onclick={() => {
                  newDomainResult = null;
                }}
                class="mt-2 text-xs text-blue-600 hover:text-blue-800 underline"
                >Dismiss</button
              >
            </div>
          {/if}

          {#if domainFeatureLocked || domainQuotaReached}
            <div
              class="bg-amber-50 border border-amber-200 rounded-lg p-3 mt-2 text-sm text-amber-800"
            >
              {#if domainFeatureLocked}
                Custom domains require the <strong>Pro plan</strong> or higher.
                <a href="/billing" class="underline font-medium"
                  >Upgrade your plan</a
                >
              {:else}
                You've reached your custom domain limit ({activeDomainCount}/
                {maxCustomDomains}). Upgrade to
                <strong>{domainUpgradeTarget()}</strong>
                to add more domains.
                <a href="/billing" class="underline font-medium">Upgrade</a>
              {/if}
            </div>
          {:else}
            <div class="flex items-center gap-2">
              <input
                type="text"
                bind:value={newHostname}
                placeholder="go.yourdomain.com"
                class="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-orange-300 font-mono"
                onkeydown={(e) => {
                  if (e.key === "Enter") handleAddDomain();
                }}
              />
              <button
                onclick={handleAddDomain}
                disabled={addingDomain || !newHostname.trim()}
                class="px-4 py-2 bg-orange-500 hover:bg-orange-600 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
              >
                {addingDomain ? "Adding..." : "Add Domain"}
              </button>
            </div>
          {/if}
          {#if addDomainError}
            <p class="text-xs text-red-600 mt-2">{addDomainError}</p>
          {/if}
        {/if}
      </div>

      <!-- Tags Management -->
      <div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-semibold text-gray-900">Tags</h2>
          <div class="flex items-center gap-2">
            <button
              onclick={toggleAnalytics}
              class="text-sm px-3 py-1.5 text-gray-600 hover:text-gray-900 border border-gray-300 rounded-lg transition-colors"
            >
              {showAnalytics ? "Hide" : "Show"} Analytics
            </button>
            <button
              onclick={openCreateTagModal}
              class="text-sm px-3 py-1.5 bg-orange-600 text-white rounded-lg hover:bg-orange-700 transition-colors"
            >
              + Create Tag
            </button>
          </div>
        </div>

        <!-- Analytics Panel -->
        {#if showAnalytics}
          <div class="mb-6 p-4 bg-gray-50 rounded-lg border border-gray-200">
            <h3 class="text-sm font-semibold text-gray-900 mb-3">
              Tag Analytics
            </h3>
            {#if analyticsLoading}
              <div class="flex items-center gap-2 text-sm text-gray-500">
                <div
                  class="animate-spin w-4 h-4 border-2 border-gray-300 border-t-gray-600 rounded-full"
                ></div>
                Loading analytics...
              </div>
            {:else if tagAnalytics}
              <div class="grid grid-cols-3 gap-4 mb-4">
                <div class="text-center p-3 bg-white rounded-lg">
                  <div class="text-2xl font-bold text-gray-900">
                    {tagAnalytics.total_tags}
                  </div>
                  <div class="text-xs text-gray-500">Total Tags</div>
                </div>
                <div class="text-center p-3 bg-white rounded-lg">
                  <div class="text-2xl font-bold text-green-600">
                    {tagAnalytics.used_tags}
                  </div>
                  <div class="text-xs text-gray-500">In Use</div>
                </div>
                <div class="text-center p-3 bg-white rounded-lg">
                  <div class="text-2xl font-bold text-amber-600">
                    {tagAnalytics.unused_tags}
                  </div>
                  <div class="text-xs text-gray-500">Unused</div>
                </div>
              </div>

              <!-- Similar Tags Suggestions -->
              {#if tagAnalytics.similar_tag_groups.length > 0}
                <div class="mb-4">
                  <h4 class="text-xs font-semibold text-gray-700 mb-2">
                    Similar Tags (Potential Merges)
                  </h4>
                  <div class="space-y-2">
                    {#each tagAnalytics.similar_tag_groups as group (group.suggestion)}
                      <div
                        class="flex items-center justify-between p-2 bg-white rounded border border-gray-200"
                      >
                        <div class="flex items-center gap-2 flex-wrap">
                          {#each group.tags as tag (tag)}
                            <span class="px-2 py-1 text-xs bg-gray-100 rounded"
                              >{tag}</span
                            >
                          {/each}
                          <span class="text-xs text-gray-500"
                            >→ Suggest: <strong>{group.suggestion}</strong
                            ></span
                          >
                        </div>
                        <button
                          onclick={() =>
                            quickMergeTags(group.tags, group.suggestion)}
                          class="text-xs px-2 py-1 bg-blue-600 text-white rounded hover:bg-blue-700"
                        >
                          Merge
                        </button>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- Unused Tags Quick Cleanup -->
              {#if tagAnalytics.unused_tag_names.length > 0}
                <div>
                  <h4 class="text-xs font-semibold text-gray-700 mb-2">
                    Unused Tags (Safe to Delete)
                  </h4>
                  <div class="flex flex-wrap gap-2">
                    {#each tagAnalytics.unused_tag_names as tagName (tagName)}
                      <span
                        class="inline-flex items-center gap-1 px-2 py-1 text-xs bg-gray-100 rounded"
                      >
                        {tagName}
                        <button
                          onclick={() => {
                            deletingTag = tagName;
                            confirmDeleteTag(tagName);
                          }}
                          class="text-gray-400 hover:text-red-600"
                          title="Delete"
                        >
                          ×
                        </button>
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}
            {:else}
              <div class="text-sm text-gray-500">Failed to load analytics.</div>
            {/if}
          </div>
        {/if}

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
            them to links, or you can create them manually above.
          </p>
        {:else}
          <!-- Search and Bulk Actions -->
          <div class="flex flex-col sm:flex-row gap-3 mb-4">
            <div class="flex-1">
              <input
                type="text"
                bind:value={tagSearchQuery}
                placeholder="Search tags..."
                class="w-full px-3 py-2 text-sm border border-gray-300 rounded-lg focus:border-orange-500 focus:outline-none"
              />
            </div>
            {#if selectedTags.size > 0}
              <div class="flex items-center gap-2">
                <span class="text-sm text-gray-600"
                  >{selectedTags.size} selected</span
                >
                <button
                  onclick={openMergeModal}
                  disabled={selectedTags.size < 2}
                  class="text-xs px-3 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Merge
                </button>
                <button
                  onclick={bulkDeleteUnusedTags}
                  class="text-xs px-3 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed"
                  disabled={!Array.from(selectedTags).some(
                    (name) => tags.find((t) => t.name === name)?.count === 0
                  )}
                >
                  Delete Unused
                </button>
                <button
                  onclick={deselectAllTags}
                  class="text-xs px-3 py-2 text-gray-600 hover:text-gray-900 border border-gray-300 rounded-lg"
                >
                  Clear
                </button>
              </div>
            {/if}
          </div>

          <!-- Tags Table -->
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-gray-200">
                  <th class="py-2 px-2 text-left w-8">
                    <input
                      type="checkbox"
                      checked={selectedTags.size === filteredTags().length &&
                        filteredTags().length > 0}
                      onclick={() =>
                        selectedTags.size === filteredTags().length
                          ? deselectAllTags()
                          : selectAllTags()}
                      class="rounded border-gray-300"
                    />
                  </th>
                  <th
                    class="py-2 px-2 text-left font-medium text-gray-700 cursor-pointer hover:text-gray-900"
                    onclick={() => setTagSort("name")}
                  >
                    Tag Name {tagSortField === "name"
                      ? tagSortDirection === "asc"
                        ? "↑"
                        : "↓"
                      : ""}
                  </th>
                  <th
                    class="py-2 px-2 text-left font-medium text-gray-700 cursor-pointer hover:text-gray-900 w-24"
                    onclick={() => setTagSort("count")}
                  >
                    Links {tagSortField === "count"
                      ? tagSortDirection === "asc"
                        ? "↑"
                        : "↓"
                      : ""}
                  </th>
                  <th
                    class="py-2 px-2 text-left font-medium text-gray-700 cursor-pointer hover:text-gray-900 w-32 hidden sm:table-cell"
                    onclick={() => setTagSort("created_at")}
                  >
                    Created {tagSortField === "created_at"
                      ? tagSortDirection === "asc"
                        ? "↑"
                        : "↓"
                      : ""}
                  </th>
                  <th
                    class="py-2 px-2 text-left font-medium text-gray-700 cursor-pointer hover:text-gray-900 w-32 hidden md:table-cell"
                    onclick={() => setTagSort("last_used_at")}
                  >
                    Last Used {tagSortField === "last_used_at"
                      ? tagSortDirection === "asc"
                        ? "↑"
                        : "↓"
                      : ""}
                  </th>
                  <th
                    class="py-2 px-2 text-right font-medium text-gray-700 w-32"
                    >Actions</th
                  >
                </tr>
              </thead>
              <tbody>
                {#each filteredTags() as tag (tag.name)}
                  <tr class="border-b border-gray-100 hover:bg-gray-50">
                    <td class="py-2 px-2">
                      <input
                        type="checkbox"
                        checked={selectedTags.has(tag.name)}
                        onclick={() => toggleTagSelection(tag.name)}
                        class="rounded border-gray-300"
                      />
                    </td>
                    <td class="py-2 px-2">
                      <div class="flex items-center gap-2">
                        <span
                          class="inline-block w-2.5 h-2.5 rounded-full {tagColor(
                            tag.name,
                            tag.color_index
                          ).split(' ')[0]}"
                        ></span>
                        <span class="font-medium text-gray-900">{tag.name}</span
                        >
                      </div>
                    </td>
                    <td class="py-2 px-2 text-gray-600">{tag.count}</td>
                    <td
                      class="py-2 px-2 text-gray-500 text-xs hidden sm:table-cell"
                    >
                      {new Date(tag.created_at * 1000).toLocaleDateString()}
                    </td>
                    <td
                      class="py-2 px-2 text-gray-500 text-xs hidden md:table-cell"
                    >
                      {tag.last_used_at
                        ? new Date(tag.last_used_at * 1000).toLocaleDateString()
                        : "Never"}
                    </td>
                    <td class="py-2 px-2 text-right">
                      <div class="flex items-center justify-end gap-1">
                        <button
                          onclick={() => openEditTagModal(tag)}
                          class="px-2 py-1 text-xs text-blue-600 hover:text-blue-700 hover:bg-blue-50 rounded"
                        >
                          Edit
                        </button>
                        <button
                          onclick={() => startDeleteTag(tag.name)}
                          class="px-2 py-1 text-xs text-red-600 hover:text-red-700 hover:bg-red-50 rounded"
                        >
                          Delete
                        </button>
                      </div>
                    </td>
                  </tr>

                  {#if deletingTag === tag.name}
                    <tr>
                      <td colspan="6" class="py-2 px-4">
                        <div
                          class="p-3 bg-red-50 border border-red-200 rounded-lg"
                        >
                          <p class="text-sm text-red-800 mb-2">
                            Delete tag "{tag.name}"?
                            {#if tag.count > 0}
                              This will remove the tag from {tag.count} link{tag.count ===
                              1
                                ? ""
                                : "s"}.
                            {/if}
                            This cannot be undone.
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
                      </td>
                    </tr>
                  {/if}
                {/each}
              </tbody>
            </table>
          </div>

          {#if filteredTags().length === 0 && tagSearchQuery}
            <div class="text-center py-8 text-gray-500">
              No tags match your search.
            </div>
          {/if}
        {/if}
      </div>

      <!-- Create Tag Modal -->
      {#if isCreateTagModalOpen}
        <div
          class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
        >
          <div class="bg-white rounded-xl max-w-md w-full p-6 shadow-2xl">
            <h3 class="text-lg font-bold text-gray-900 mb-4">Create New Tag</h3>

            <div class="mb-4">
              <label
                for="newTagName"
                class="block text-sm font-medium text-gray-700 mb-1"
                >Tag Name</label
              >
              <input
                id="newTagName"
                type="text"
                bind:value={newTagNameInput}
                placeholder="Enter tag name..."
                maxlength="50"
                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:border-orange-500 focus:outline-none"
              />
              <p class="text-xs text-gray-500 mt-1">Max 50 characters</p>
            </div>

            <div class="mb-4">
              <span class="block text-sm font-medium text-gray-700 mb-2"
                >Color (optional)</span
              >
              <div class="flex gap-2 flex-wrap">
                {#each TAG_COLORS as color, index (index)}
                  <button
                    onclick={() =>
                      (selectedColorIndex =
                        selectedColorIndex === index ? null : index)}
                    class="w-8 h-8 rounded-full {color.split(
                      ' '
                    )[0]} {selectedColorIndex === index
                      ? 'ring-2 ring-offset-2 ring-gray-400'
                      : ''}"
                    title="Select color"
                  ></button>
                {/each}
              </div>
            </div>

            {#if createTagError}
              <div class="mb-4 text-sm text-red-600">{createTagError}</div>
            {/if}

            <div class="flex gap-3">
              <button
                onclick={closeCreateTagModal}
                class="flex-1 px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Cancel
              </button>
              <button
                onclick={confirmCreateTag}
                disabled={creatingTag || !newTagNameInput.trim()}
                class="flex-1 px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 disabled:opacity-50"
              >
                {creatingTag ? "Creating..." : "Create Tag"}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Edit Tag Modal -->
      {#if isEditTagModalOpen}
        <div
          class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
        >
          <div class="bg-white rounded-xl max-w-md w-full p-6 shadow-2xl">
            <h3 class="text-lg font-bold text-gray-900 mb-4">Edit Tag</h3>

            <div class="mb-4">
              <label
                for="editTagName"
                class="block text-sm font-medium text-gray-700 mb-1"
                >Tag Name</label
              >
              <input
                id="editTagName"
                type="text"
                bind:value={editTagNewName}
                placeholder="Enter tag name..."
                maxlength="50"
                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:border-orange-500 focus:outline-none"
              />
              <p class="text-xs text-gray-500 mt-1">Max 50 characters</p>
            </div>

            <div class="mb-4">
              <span class="block text-sm font-medium text-gray-700 mb-2"
                >Color</span
              >
              <div class="flex gap-2 flex-wrap">
                {#each TAG_COLORS as color, index (index)}
                  <button
                    onclick={() =>
                      (editTagColorIndex =
                        editTagColorIndex === index ? null : index)}
                    class="w-8 h-8 rounded-full {color.split(
                      ' '
                    )[0]} {editTagColorIndex === index
                      ? 'ring-2 ring-offset-2 ring-gray-400'
                      : ''}"
                    title="Select color"
                  ></button>
                {/each}
              </div>
              {#if editTagColorIndex === null}
                <p class="text-xs text-gray-500 mt-2">
                  No color selected. Will use auto-generated color based on tag
                  name.
                </p>
              {/if}
            </div>

            {#if editTagError}
              <div class="mb-4 text-sm text-red-600">{editTagError}</div>
            {/if}

            <div class="flex gap-3">
              <button
                onclick={closeEditTagModal}
                class="flex-1 px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Cancel
              </button>
              <button
                onclick={confirmEditTag}
                disabled={savingEditTag || !editTagNewName.trim()}
                class="flex-1 px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 disabled:opacity-50"
              >
                {savingEditTag ? "Saving..." : "Save Changes"}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Merge Tags Modal -->
      {#if isMergeModalOpen}
        <div
          class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
        >
          <div class="bg-white rounded-xl max-w-md w-full p-6 shadow-2xl">
            <h3 class="text-lg font-bold text-gray-900 mb-2">Merge Tags</h3>
            <p class="text-sm text-gray-600 mb-4">
              Merge {selectedTags.size} selected tags into one destination tag. All
              links using the source tags will be updated.
            </p>

            <div class="mb-4">
              <label
                for="mergeDestination"
                class="block text-sm font-medium text-gray-700 mb-2"
                >Destination Tag</label
              >
              <select
                id="mergeDestination"
                bind:value={mergeDestinationTag}
                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:border-orange-500 focus:outline-none"
              >
                <option value="">Select destination...</option>
                {#each Array.from(selectedTags) as tagName (tagName)}
                  <option value={tagName}>{tagName}</option>
                {/each}
                <option value="__new__">-- Create new tag --</option>
              </select>
              {#if mergeDestinationTag === "__new__"}
                <input
                  type="text"
                  bind:value={mergeDestinationTag}
                  placeholder="Enter new tag name..."
                  class="w-full mt-2 px-3 py-2 border border-gray-300 rounded-lg focus:border-orange-500 focus:outline-none"
                  onfocus={() => {
                    if (mergeDestinationTag === "__new__")
                      mergeDestinationTag = "";
                  }}
                />
              {/if}
            </div>

            <div class="mb-4 p-3 bg-blue-50 rounded-lg">
              <p class="text-sm text-blue-800">
                <strong>Source tags to merge:</strong>
                {#each Array.from(selectedTags).filter((t) => t !== mergeDestinationTag) as tag, i (tag)}
                  {i > 0 ? ", " : ""}"{tag}"
                {/each}
              </p>
            </div>

            {#if mergeError}
              <div class="mb-4 text-sm text-red-600">{mergeError}</div>
            {/if}

            <div class="flex gap-3">
              <button
                onclick={closeMergeModal}
                class="flex-1 px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Cancel
              </button>
              <button
                onclick={confirmMergeTags}
                disabled={mergingTags}
                class="flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                {mergingTags ? "Merging..." : "Merge Tags"}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Danger Zone (owner only) -->
      {#if isOwner}
        <div class="bg-white rounded-xl border border-red-200 p-6">
          <h2 class="text-lg font-semibold text-red-700 mb-2">Danger Zone</h2>
          <p class="text-sm text-gray-600 mb-4">
            Deleting an organization is permanent and cannot be undone.
          </p>
          <button
            onclick={openDeleteModal}
            disabled={!canDelete}
            class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Delete Organization
          </button>
          {#if !canDelete}
            <p class="text-xs text-gray-500 mt-2">
              You must own at least one other organization to delete this one.
              To delete everything, use <a
                href="/settings"
                class="text-orange-600 hover:underline">account deletion</a
              > in Settings.
            </p>
          {/if}
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

<!-- Domain Deletion Confirmation Modal -->
{#if confirmingDomainHostname}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={closeConfirmDomain}
    onkeydown={(e) => e.key === "Enter" && closeConfirmDomain()}
  >
    <div
      class="modal"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="0"
      onkeydown={(e) => e.key === "Escape" && closeConfirmDomain()}
    >
      <div class="modal-header">
        <h3>Remove Custom Domain?</h3>
        <button class="modal-close" onclick={closeConfirmDomain}>&times;</button
        >
      </div>
      <div class="modal-body">
        <p>
          Remove custom domain "{confirmingDomainHostname}"? All short links
          served through this domain will stop working. This action cannot be
          undone.
        </p>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={closeConfirmDomain}>
          Cancel
        </button>
        <button class="btn btn-danger" onclick={confirmDeleteDomain}>
          Remove
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
