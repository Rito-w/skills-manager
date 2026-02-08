import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { homeDir, join } from "@tauri-apps/api/path";
import { useToast } from "./useToast";

export type RemoteSkill = {
  id: string;
  name: string;
  namespace: string;
  sourceUrl: string;
  description: string;
  author: string;
  installs: number;
  stars: number;
  marketId: string;
  marketLabel: string;
};

export type MarketStatus = {
  id: string;
  name: string;
  status: "online" | "error" | "needs_key";
  error?: string;
};

export type InstallResult = {
  installedPath: string;
  linked: string[];
  skipped: string[];
};

export type LocalSkill = {
  id: string;
  name: string;
  description: string;
  path: string;
  source: string;
  ide?: string;
  usedBy: string[];
};

export type IdeSkill = {
  id: string;
  name: string;
  path: string;
  ide: string;
  source: string;
};

export type Overview = {
  managerSkills: LocalSkill[];
  ideSkills: IdeSkill[];
};

export type IdeOption = {
  id: string;
  label: string;
  globalDir: string;
};

type LinkTarget = {
  name: string;
  path: string;
};

const defaultIdeOptions: IdeOption[] = [
  { id: "antigravity", label: "Antigravity", globalDir: ".agent/skills" },
  { id: "claude", label: "Claude", globalDir: ".claude/skills" },
  { id: "codebuddy", label: "CodeBuddy", globalDir: ".codebuddy/skills" },
  { id: "codex", label: "Codex", globalDir: ".codex/skills" },
  { id: "cursor", label: "Cursor", globalDir: ".cursor/skills" },
  { id: "kiro", label: "Kiro", globalDir: ".kiro/skills" },
  { id: "qoder", label: "Qoder", globalDir: ".qoder/skills" },
  { id: "trae", label: "Trae", globalDir: ".trae/skills" },
  { id: "vscode", label: "VSCode", globalDir: ".github/skills" },
  { id: "windsurf", label: "Windsurf", globalDir: ".windsurf/skills" }
];

const ideKey = "skillsManager.ideOptions";
const installTargetKey = "skillsManager.lastInstallTargets";
const marketConfigsKey = "skillsManager.marketConfigs";

function loadIdeOptions(): IdeOption[] {
  try {
    const raw = localStorage.getItem(ideKey);
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

function saveIdeOptions(custom: IdeOption[]) {
  localStorage.setItem(ideKey, JSON.stringify(custom));
}

function loadLastInstallTargets(): string[] {
  try {
    const raw = localStorage.getItem(installTargetKey);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item) => typeof item === "string");
  } catch {
    return [];
  }
}

function saveLastInstallTargets(labels: string[]) {
  localStorage.setItem(installTargetKey, JSON.stringify(labels));
}

export function useSkillsManager() {
  const { t } = useI18n();
  const toast = useToast();
  const cacheTtlMs = 10 * 60 * 1000;
  const searchCache = new Map<
    string,
    { timestamp: number; data: { skills: RemoteSkill[]; total: number; limit: number; offset: number; marketStatuses: MarketStatus[] } }
  >();
  const activeTab = ref<"local" | "market" | "ide">("local");

  const query = ref("");
  const results = ref<RemoteSkill[]>([]);
  const total = ref(0);
  const limit = ref(20);
  const offset = ref(0);
  const loading = ref(false);
  const installingId = ref<string | null>(null);
  const updatingId = ref<string | null>(null);

  const marketConfigs = ref<Record<string, string>>({});
  // Track which markets are enabled by the user
  const enabledMarkets = ref<Record<string, boolean>>({
    "claude-plugins": true,
    "skillsllm": true,
    "skillsmp": false // Disabled by default until API key is provided
  });
  const marketStatuses = ref<MarketStatus[]>([
    { id: "claude-plugins", name: "Claude Plugins", status: "online" },
    { id: "skillsllm", name: "SkillsLLM", status: "online" },
    { id: "skillsmp", name: "SkillsMP", status: "needs_key" }
  ]);

  // Local Skills
  const localSkills = ref<LocalSkill[]>([]);
  const ideSkills = ref<IdeSkill[]>([]);
  const localLoading = ref(false);

  const ideOptions = ref<IdeOption[]>([]);
  const selectedIdeFilter = ref("Antigravity");
  const customIdeName = ref("");
  const customIdeDir = ref("");

  const showInstallModal = ref(false);
  const installTargetSkill = ref<LocalSkill | null>(null);
  const installTargetIde = ref<string[]>([]);

  const showUninstallModal = ref(false);
  const uninstallTargetPath = ref("");
  const uninstallTargetName = ref("");

  const busy = ref(false);
  const busyText = ref("");

  const hasMore = computed(() => results.value.length < total.value);
  const localSkillNameSet = computed(() => {
    const set = new Set<string>();
    for (const skill of localSkills.value) {
      const key = skill.name.trim().toLowerCase();
      if (key) set.add(key);
    }
    return set;
  });



  const filteredIdeSkills = computed(() =>
    ideSkills.value.filter((skill) => skill.ide === selectedIdeFilter.value)
  );

  const customIdeOptions = computed(() =>
    ideOptions.value.filter((item) => item.id.startsWith("custom-"))
  );

  function loadMarketConfigs() {
    const saved = localStorage.getItem(marketConfigsKey);
    if (saved) {
      try {
        marketConfigs.value = JSON.parse(saved);
      } catch (e) {
        console.error("Failed to parse marketConfigs", e);
      }
    }
    // Load enabled markets
    const savedEnabled = localStorage.getItem('market-enabled');
    if (savedEnabled) {
      try {
        enabledMarkets.value = JSON.parse(savedEnabled);
      } catch (e) {
        console.error("Failed to parse enabledMarkets", e);
      }
    }
  }

  function saveMarketConfigs(configs: Record<string, string>, enabled: Record<string, boolean>) {
    marketConfigs.value = configs;
    enabledMarkets.value = enabled;
    localStorage.setItem(marketConfigsKey, JSON.stringify(configs));
    localStorage.setItem('market-enabled', JSON.stringify(enabled));
  }

  function refreshIdeOptions() {
    ideOptions.value = loadIdeOptions();
    if (!ideOptions.value.find((item) => item.label === selectedIdeFilter.value)) {
      selectedIdeFilter.value = ideOptions.value[0]?.label ?? "Antigravity";
    }
  }

  function addCustomIde() {
    const name = customIdeName.value.trim();
    const dir = customIdeDir.value.trim();
    if (!name || !dir) {
      toast.error(t("errors.fillIde"));
      return;
    }
    const normalizedName = name.toLowerCase();
    if (ideOptions.value.some((item) => item.label.toLowerCase() === normalizedName)) {
      toast.error(t("errors.ideExists"));
      return;
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
    void scanLocalSkills();
  }

  function removeCustomIde(label: string) {
    const customOnly = ideOptions.value.filter(
      (item) => !defaultIdeOptions.find((def) => def.id === item.id)
    );
    const nextCustom = customOnly.filter((item) => item.label !== label);
    saveIdeOptions(nextCustom);
    refreshIdeOptions();
    void scanLocalSkills();
  }

  async function buildInstallBaseDir(): Promise<string> {
    const home = await homeDir();
    return join(home, ".skills-manager/skills");
  }

  async function buildLinkTargets(targetLabel: string): Promise<LinkTarget[]> {
    const home = await homeDir();
    const target = ideOptions.value.find((option) => option.label === targetLabel);
    if (!target) return [];
    return [
      {
        name: target.label,
        path: await join(home, target.globalDir)
      }
    ];
  }

  async function searchMarketplace(reset = true, force = false) {
    if (loading.value) return;
    loading.value = true;

    const nextOffset = reset ? 0 : offset.value + limit.value;
    const cacheKey = `${query.value.trim().toLowerCase()}|${limit.value}`;

    if (reset && !force) {
      const cached = searchCache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < cacheTtlMs) {
        results.value = cached.data.skills;
        total.value = cached.data.total;
        offset.value = cached.data.offset;
        marketStatuses.value = cached.data.marketStatuses;
        loading.value = false;
        return;
      }
    }

    try {
      const response = await invoke("search_marketplaces", {
        query: query.value,
        limit: limit.value,
        offset: nextOffset,
        apiKeys: marketConfigs.value,
        enabledMarkets: enabledMarkets.value
      });
      const data = response as {
        skills: RemoteSkill[];
        total: number;
        limit: number;
        offset: number;
        marketStatuses: MarketStatus[];
      };

      const deduped = dedupeSkills(reset ? data.skills : [...results.value, ...data.skills]);
      results.value = deduped;

      total.value = data.total;
      offset.value = data.offset;
      if (Array.isArray(data.marketStatuses)) {
        marketStatuses.value = data.marketStatuses;
      }

      if (reset) {
        const cachedStatuses = Array.isArray(data.marketStatuses)
          ? data.marketStatuses
          : marketStatuses.value;
        searchCache.set(cacheKey, {
          timestamp: Date.now(),
          data: { ...data, marketStatuses: cachedStatuses }
        });
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.searchFailed"));
    } finally {
      loading.value = false;
    }
  }

  function dedupeSkills(skills: RemoteSkill[]) {
    const map = new Map<string, RemoteSkill>();
    for (const skill of skills) {
      const sourceKey = skill.sourceUrl?.trim().toLowerCase();
      const nameKey = `${skill.marketId}:${skill.name.trim().toLowerCase()}`;
      const key = sourceKey || nameKey;
      if (!map.has(key)) {
        map.set(key, skill);
      }
    }
    return Array.from(map.values());
  }

  async function downloadSkill(skill: RemoteSkill) {
    if (installingId.value) return;

    installingId.value = skill.id;
    busy.value = true;
    busyText.value = t("market.downloading");

    try {
      const installBaseDir = await buildInstallBaseDir();
      const result = (await invoke("download_marketplace_skill", {
        request: {
          sourceUrl: skill.sourceUrl,
          skillName: skill.name,
          installBaseDir
        }
      })) as { installedPath: string };

      toast.success(t("messages.downloaded", { path: result.installedPath }));
      await scanLocalSkills();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.downloadFailed"));
    } finally {
      installingId.value = null;
      busy.value = false;
      busyText.value = "";
    }
  }

  async function updateSkill(skill: RemoteSkill) {
    if (updatingId.value) return;

    updatingId.value = skill.id;
    busy.value = true;
    busyText.value = t("market.updating");

    try {
      const installBaseDir = await buildInstallBaseDir();
      const result = (await invoke("update_marketplace_skill", {
        request: {
          sourceUrl: skill.sourceUrl,
          skillName: skill.name,
          installBaseDir
        }
      })) as { installedPath: string };

      toast.success(t("messages.updated", { path: result.installedPath }));
      await scanLocalSkills();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.updateFailed"));
    } finally {
      updatingId.value = null;
      busy.value = false;
      busyText.value = "";
    }
  }

  async function scanLocalSkills() {
    if (localLoading.value) return;
    localLoading.value = true;

    try {
      const response = (await invoke("scan_overview", {
        request: {
          projectDir: null,
          ideDirs: ideOptions.value.map((item) => ({
            label: item.label,
            relativeDir: item.globalDir
          }))
        }
      })) as Overview;
      localSkills.value = response.managerSkills;
      ideSkills.value = response.ideSkills;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.scanFailed"));
    } finally {
      localLoading.value = false;
    }
  }

  async function linkSkillInternal(skill: LocalSkill, ideLabel: string, skipScan = false, suppressToast = false) {
    const linkTargets = await buildLinkTargets(ideLabel);
    if (linkTargets.length === 0) {
      throw new Error(t("errors.selectValidIde"));
    }
    const result = (await invoke("link_local_skill", {
      request: {
        skillPath: skill.path,
        skillName: skill.name,
        linkTargets
      }
    })) as InstallResult;

    const linkedCount = result.linked.length;
    const skippedCount = result.skipped.length;
    if (!suppressToast) {
      toast.success(t("messages.handled", { linked: linkedCount, skipped: skippedCount }));
    }
    if (!skipScan) {
      await scanLocalSkills();
    }
    return result;
  }

  function openInstallModal(skill: LocalSkill) {
    installTargetSkill.value = skill;
    const lastTargets = loadLastInstallTargets();
    const available = new Set(ideOptions.value.map((item) => item.label));
    const nextTargets = lastTargets.filter((label) => available.has(label));
    installTargetIde.value = nextTargets;
    showInstallModal.value = true;
  }

  function updateInstallTargetIde(next: string[]) {
    installTargetIde.value = next;
    saveLastInstallTargets(next);
  }

  async function confirmInstallToIde() {
    if (!installTargetSkill.value || installTargetIde.value.length === 0) {
      toast.error(t("errors.selectAtLeastOne"));
      return;
    }
    if (installingId.value) return;
    installingId.value = installTargetSkill.value.id;
    busy.value = true;
    busyText.value = t("messages.installing");

    try {
      let totalLinked = 0;
      let totalSkipped = 0;

      for (const label of installTargetIde.value) {
        const result = await linkSkillInternal(installTargetSkill.value, label, true, true);
        totalLinked += result.linked.length;
        totalSkipped += result.skipped.length;
      }
      toast.success(t("messages.handled", { linked: totalLinked, skipped: totalSkipped }));
      await scanLocalSkills();
      showInstallModal.value = false;
      installTargetSkill.value = null;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.installFailed"));
    } finally {
      installingId.value = null;
      busy.value = false;
      busyText.value = "";
    }
  }

  function closeInstallModal() {
    showInstallModal.value = false;
    installTargetSkill.value = null;
  }

  function openUninstallModal(targetPath: string) {
    uninstallTargetPath.value = targetPath;
    uninstallTargetName.value = targetPath.split("/").pop() || targetPath;
    showUninstallModal.value = true;
  }

  async function confirmUninstall() {
    busy.value = true;
    busyText.value = t("messages.uninstalling");
    try {
      const message = (await invoke("uninstall_skill", {
        request: {
          targetPath: uninstallTargetPath.value,
          projectDir: null,
          ideDirs: ideOptions.value.map((item) => ({
            label: item.label,
            relativeDir: item.globalDir
          }))
        }
      })) as string;
      toast.success(message);
      await scanLocalSkills();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.uninstallFailed"));
    } finally {
      showUninstallModal.value = false;
      uninstallTargetPath.value = "";
      uninstallTargetName.value = "";
      busy.value = false;
      busyText.value = "";
    }
  }

  function cancelUninstall() {
    showUninstallModal.value = false;
    uninstallTargetPath.value = "";
    uninstallTargetName.value = "";
  }

  async function importLocalSkill() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: true,
        title: t("local.selectSkillDir")
      });

      if (!selected) return;

      const paths = Array.isArray(selected) ? selected : [selected];
      if (paths.length === 0) return;

      busy.value = true;
      busyText.value = t("messages.importing");

      let successCount = 0;
      let failCount = 0;
      let lastError = "";

      for (const path of paths) {
        try {
          await invoke("import_local_skill", {
            request: {
              sourcePath: path
            }
          });
          successCount++;
        } catch (err) {
          failCount++;
          lastError = err instanceof Error ? err.message : String(err);
        }
      }

      if (successCount > 0) {
        toast.success(t("messages.imported", { success: successCount, failed: failCount }));
      } else {
        toast.error(
          t("messages.imported", { success: 0, failed: failCount }) +
          (paths.length === 1 ? `: ${lastError}` : "")
        );
      }

      await scanLocalSkills();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.importFailed"));
    } finally {
      busy.value = false;
      busyText.value = "";
    }
  }

  onMounted(() => {
    refreshIdeOptions();
    loadMarketConfigs();
    void searchMarketplace(true);
    void scanLocalSkills();
  });

  return {
    // State
    activeTab,
    query,
    results,
    total,
    limit,
    offset,
    loading,
    installingId,
    updatingId,
    localSkills,
    ideSkills,
    localLoading,
    ideOptions,
    selectedIdeFilter,
    customIdeName,
    customIdeDir,
    showInstallModal,
    installTargetIde,
    showUninstallModal,
    uninstallTargetName,
    busy,
    busyText,
    hasMore,
    localSkillNameSet,
    filteredIdeSkills,
    customIdeOptions,
    marketConfigs,
    marketStatuses,
    enabledMarkets,

    // Actions
    refreshIdeOptions,
    addCustomIde,
    removeCustomIde,
    saveMarketConfigs,
    searchMarketplace,
    downloadSkill,
    updateSkill,
    scanLocalSkills,
    openInstallModal,
    updateInstallTargetIde,
    confirmInstallToIde,
    closeInstallModal,
    openUninstallModal,
    confirmUninstall,
    cancelUninstall,
    importLocalSkill
  };
}
