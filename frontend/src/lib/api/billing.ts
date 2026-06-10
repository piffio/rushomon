import type { InitiateTransferResponse, TransferInfo } from "$lib/types/api";
import { apiClient } from "./client";

export interface BillingStatus {
  tier: string;
  is_billing_owner: boolean;
  interval: string | null;
  amount_cents: number | null;
  currency: string | null;
  discount_name: string | null;
  provider_customer_id: string | null;
  billing_account_id: string | null;
  subscription_id: string | null;
  subscription_status: string | null;
  subscription_plan: string | null;
  current_period_end: number | null;
  cancel_at_period_end: boolean;
}

/** One entry from GET /api/billing/accounts */
export interface BillingAccountSummary {
  id: string;
  tier: string;
  is_billing_owner: true;
  subscription_status: string | null;
  current_period_end: number | null;
  cancel_at_period_end: boolean;
  amount_cents: number | null;
  currency: string | null;
  interval: string | null;
  organizations: Array<{
    id: string;
    name: string;
    slug: string;
    member_count: number;
    link_count: number;
    created_at: number;
  }>;
}

export interface BillingAccountsResponse {
  accounts: BillingAccountSummary[];
}

export interface CheckoutRequest {
  plan: string; // e.g., "pro_monthly", "business_annual"
}

export interface CheckoutResponse {
  url: string;
}

export interface PortalResponse {
  url: string;
}

export interface ProductPrice {
  id: string;
  polar_product_id: string;
  polar_price_id: string;
  name: string;
  price_amount: number;
  price_currency: string;
  recurring_interval: string | null;
  recurring_interval_count: number | null;
}

export interface PricingResponse {
  products: ProductPrice[];
}

export const billingApi = {
  /** All billing accounts owned by the current user, highest-tier first. */
  getAccounts(): Promise<BillingAccountsResponse> {
    return apiClient.get<BillingAccountsResponse>("/api/billing/accounts");
  },

  getStatus(): Promise<BillingStatus> {
    return apiClient.get<BillingStatus>("/api/billing/status");
  },

  getPricing(): Promise<PricingResponse> {
    return apiClient.get<PricingResponse>("/api/billing/pricing");
  },

  async createCheckout(plan: string): Promise<CheckoutResponse> {
    const requestBody: CheckoutRequest = {
      plan
    };

    const res = await apiClient.post<CheckoutResponse>(
      "/api/billing/checkout",
      requestBody
    );

    if (!res.url) {
      throw new Error("No checkout URL returned");
    }

    return { url: res.url };
  },

  async createPortal(): Promise<PortalResponse> {
    const res = await apiClient.post<PortalResponse>("/api/billing/portal", {});

    if (!res.url) {
      throw new Error("No portal URL returned");
    }

    return { url: res.url };
  },

  // ─── Ownership Transfer ───────────────────────────────────────────────────

  /**
   * Initiate a billing account ownership transfer.
   * Only the billing account owner can call this.
   * @param toEmail - Email of the intended recipient (must be a member of one of the BA's orgs)
   * @param billingAccountId - Optional: explicit BA ID (defaults to caller's own BA)
   */
  initiateTransfer(
    toEmail: string,
    billingAccountId?: string
  ): Promise<InitiateTransferResponse> {
    return apiClient.post<InitiateTransferResponse>("/api/billing/transfer", {
      to_email: toEmail,
      billing_account_id: billingAccountId
    });
  },

  /**
   * Cancel the outstanding ownership transfer for the caller's billing account.
   */
  cancelTransfer(): Promise<{ success: boolean; message: string }> {
    return apiClient.request<{ success: boolean; message: string }>(
      "/api/billing/transfer",
      { method: "DELETE" }
    );
  },

  /**
   * Fetch public information about a pending transfer (no auth required).
   * @param token - Transfer token from the email link
   */
  getTransferInfo(token: string): Promise<TransferInfo> {
    return apiClient.get<TransferInfo>(`/api/billing-transfer/${token}`);
  },

  /**
   * Accept a pending ownership transfer.
   * The caller must be logged in and their email must match the transfer's to_email.
   * @param token - Transfer token from the email link
   */
  acceptTransfer(
    token: string
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.post<{ success: boolean; message: string }>(
      `/api/billing-transfer/${token}/accept`,
      {}
    );
  }
};
