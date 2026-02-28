import { ref } from "vue";
import type { MarketStatus } from "./types";
import { STORAGE_KEYS, defaultMarketStatuses, defaultEnabledMarkets } from "./constants";

/**
 * Market configuration management composable
 */
export function useMarketConfig() {
  const marketConfigs = ref<Record<string, string>>({});
  const enabledMarkets = ref<Record<string, boolean>>({ ...defaultEnabledMarkets });
  const marketStatuses = ref<MarketStatus[]>([...defaultMarketStatuses]);

  /**
   * Load market configurations from localStorage
   */
  function loadMarketConfigs(): void {
    const saved = localStorage.getItem(STORAGE_KEYS.marketConfigs);
    if (saved) {
      try {
        marketConfigs.value = JSON.parse(saved);
      } catch (e) {
        console.error("Failed to parse marketConfigs", e);
      }
    }
    // Load enabled markets
    const savedEnabled = localStorage.getItem(STORAGE_KEYS.enabledMarkets);
    if (savedEnabled) {
      try {
        enabledMarkets.value = JSON.parse(savedEnabled);
      } catch (e) {
        console.error("Failed to parse enabledMarkets", e);
      }
    }
  }

  /**
   * Save market configurations to localStorage
   */
  function saveMarketConfigs(configs: Record<string, string>, enabled: Record<string, boolean>): void {
    marketConfigs.value = configs;
    enabledMarkets.value = enabled;
    localStorage.setItem(STORAGE_KEYS.marketConfigs, JSON.stringify(configs));
    localStorage.setItem(STORAGE_KEYS.enabledMarkets, JSON.stringify(enabled));
  }

  /**
   * Update market statuses from API response
   */
  function updateMarketStatuses(statuses: MarketStatus[]): void {
    if (Array.isArray(statuses)) {
      marketStatuses.value = statuses;
    }
  }

  return {
    // State
    marketConfigs,
    enabledMarkets,
    marketStatuses,

    // Actions
    loadMarketConfigs,
    saveMarketConfigs,
    updateMarketStatuses
  };
}
