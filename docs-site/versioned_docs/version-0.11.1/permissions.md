---
sidebar_position: 2
---

# Permissions & Tiers

Rushomon uses a two-level permission model: **organization roles** (within an org) and **billing tiers** (feature limits). This page explains both.

## Organization Roles

Every user in an organization has one of three roles:

| Role | Description |
|------|-------------|
| **Owner** | Full control over the organization. Can delete the org, manage billing, and change any member's role. The owner is the only person who can delete the organization. |
| **Admin** | Can invite members, manage org settings, and promote/demote regular members. Cannot delete the org, change owner roles, or modify other admins. |
| **Member** | Can create, edit, and delete links; view analytics. Cannot invite members, change settings, or manage custom domains. |

### Permission Matrix

| Operation | Owner | Admin | Member |
|-----------|-------|-------|--------|
| View org, list links, view analytics | ✅ | ✅ | ✅ |
| Create / edit / delete links | ✅ | ✅ | ✅ |
| Rename org | ✅ | ✅ | ❌ |
| Manage org settings | ✅ | ✅ | ❌ |
| Invite members (choose member or admin role) | ✅ | ✅ | ❌ |
| Revoke / resend invitations | ✅ | ✅ | ❌ |
| Remove members (members & admins, not owners) | ✅ | ✅ (non-owner targets) | ❌ (self only) |
| Update member role (member ↔ admin) | ✅ | ✅ (can't touch owners or other admins) | ❌ |
| Manage custom domains | ✅ | ✅ | ❌ |
| Delete org | ✅ | ❌ | ❌ |
| Billing / subscription changes | ✅ (owner-only) | ❌ | ❌ |

### Key Rules

- **Owner cannot be removed** via the member removal endpoint. The last owner of an org cannot be removed at all.
- **Admins cannot change other admins' roles** — only owners can demote admins.
- **Admins cannot remove owners or other admins** — they can only remove regular members.
- **Nobody can assign the 'owner' role** via the role-update endpoint. Ownership transfer is a separate concern (not currently implemented).
- **Billing/subscription changes are owner-only** — this is enforced at the service level because billing accounts are linked to the owner's user account.

## Billing Tiers

Rushomon offers four billing tiers that determine feature access and quotas:

| Feature | Free | Pro | Business | Unlimited |
|---------|------|-----|----------|-----------|
| Links/month | 15 | 1,000 | 10,000 | ∞ |
| Analytics retention | 7 days | 365 days | ∞ | ∞ |
| Custom short codes | ❌ | ✅ | ✅ | ✅ |
| UTM parameters | ❌ | ✅ | ✅ | ✅ |
| Query forwarding | ❌ | ✅ | ✅ | ✅ |
| Device routing | ❌ | ❌ | ✅ | ✅ |
| API keys | ❌ | ✅ | ✅ | ✅ |
| Max members | 1 | 1 | 20 | ∞ |
| Max orgs | 1 | 1 | 3 | ∞ |
| Max tags | 5 | 25 | ∞ | ∞ |
| Custom domains | 0 | 1 | 3 | ∞ |

### Tier Enforcement

- **Link creation**: Checked against the monthly link quota for the org's tier.
- **Member invitations**: Checked against the member limit (Business tier allows up to 20 members; Unlimited has no limit).
- **API key creation**: Only available on Pro tier and higher.
- **Analytics queries**: Free tier returns only the last 7 days of data; higher tiers have longer or unlimited retention.
- **Custom domains**: Tier limits the number of domains per org.

### Self-Hosting

If you're self-hosting Rushomon, you can disable all tier limits by setting the `default_user_tier` system setting to `'unlimited'` in the database. This removes all quotas and feature restrictions.

## Instance-Level vs Organization-Level

Rushomon has a two-level permission model:

1. **Instance-level roles** (`users.role` in the database):
   - `admin`: Full system access (can manage all users, billing accounts, system settings, etc.)
   - `member`: Regular user (default)

2. **Organization-level roles** (`org_members.role` in the database):
   - `owner`: Full org control
   - `admin`: Org management (but not deletion)
   - `member`: Regular org member

Instance-level admins are intended for system administration (e.g., managing the managed service). Organization-level roles control access within a specific organization.
