import { computed, ref } from "vue";
import type { IdeOption } from "./types";
import { defaultIdeOptions, STORAGE_KEYS } from "./constants";

/**
 * Load IDE options from localStorage
 */
function loadIdeOptions(): IdeOption[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.ideOptions);
    if (!raw) return [...defaultIdeOptions];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [...defaultIdeOptions];
    const custom = parsed.filter(
      (item) =>
        item &&
        typeof item.label === "string" &&
        typeof item.globalDir === "string"
    );
    return [...defaultIdeOptions, ...custom].sort((a, b) => a.label.localeCompare(b.label));
  } catch {
    return [...defaultIdeOptions];
  }
}

/**
 * Save custom IDE options to localStorage
 */
function saveIdeOptions(custom: IdeOption[]): void {
  localStorage.setItem(STORAGE_KEYS.ideOptions, JSON.stringify(custom));
}

/**
 * Load last install targets from localStorage
 */
export function loadLastInstallTargets(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.installTargets);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item) => typeof item === "string");
  } catch {
    return [];
  }
}

/**
 * Save last install targets to localStorage
 */
export function saveLastInstallTargets(labels: string[]): void {
  localStorage.setItem(STORAGE_KEYS.installTargets, JSON.stringify(labels));
}

/**
 * IDE configuration management composable
 */
export function useIdeConfig() {
  const ideOptions = ref<IdeOption[]>([]);
  const selectedIdeFilter = ref("Antigravity");
  const customIdeName = ref("");
  const customIdeDir = ref("");

  const customIdeOptions = computed(() =>
    ideOptions.value.filter((item) => item.id.startsWith("custom-"))
  );

  function refreshIdeOptions(): void {
    ideOptions.value = loadIdeOptions();
    if (!ideOptions.value.find((item) => item.label === selectedIdeFilter.value)) {
      selectedIdeFilter.value = ideOptions.value[0]?.label ?? "Antigravity";
    }
  }

  function addCustomIde(t: (key: string) => string, onError: (msg: string) => void): boolean {
    const name = customIdeName.value.trim();
    const dir = customIdeDir.value.trim();
    if (!name || !dir) {
      onError(t("errors.fillIde"));
      return false;
    }
    const normalizedName = name.toLowerCase();
    if (ideOptions.value.some((item) => item.label.toLowerCase() === normalizedName)) {
      onError(t("errors.ideExists"));
      return false;
    }
    const existingCustom = ideOptions.value
      .filter((item) => !defaultIdeOptions.find((def) => def.id === item.id))
      .filter((item) => item.label.toLowerCase() !== normalizedName);
    const id = `custom-${name.toLowerCase().replace(/\s+/g, "-")}`;
    const nextCustom = [...existingCustom, { id, label: name, globalDir: dir }].sort((a, b) =>
      a.label.localeCompare(b.label)
    );
    saveIdeOptions(nextCustom);
    customIdeName.value = "";
    customIdeDir.value = "";
    refreshIdeOptions();
    return true;
  }

  function removeCustomIde(label: string): void {
    const customOnly = ideOptions.value.filter(
      (item) => !defaultIdeOptions.find((def) => def.id === item.id)
    );
    const nextCustom = customOnly.filter((item) => item.label !== label);
    saveIdeOptions(nextCustom);
    refreshIdeOptions();
  }

  return {
    // State
    ideOptions,
    selectedIdeFilter,
    customIdeName,
    customIdeDir,
    customIdeOptions,

    // Actions
    refreshIdeOptions,
    addCustomIde,
    removeCustomIde,
    loadLastInstallTargets,
    saveLastInstallTargets
  };
}
