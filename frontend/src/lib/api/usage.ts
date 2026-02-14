import { apiClient } from './client';
import type { UsageResponse } from '$lib/types/api';

export const usageApi = {
	/**
	 * Get current tier and usage info for the authenticated org
	 * @returns Usage response with tier, limits, and current usage
	 */
	async getUsage(): Promise<UsageResponse> {
		return apiClient.get<UsageResponse>('/api/usage');
	}
};
