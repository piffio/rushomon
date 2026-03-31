import type { PageLoad } from "./$types";
import type { User } from "$lib/types/api";

export const prerender = true;
export const ssr = true;

export const load: PageLoad = async () => {
  // For pre-rendered report page, we don't need user data
  // This page should be accessible to everyone including Google's validation bots
  return {
    user: undefined as User | undefined
  };
};
