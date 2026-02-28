import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { homeDir, join } from "@tauri-apps/api/path";
import { useToast } from "./useToast";
import type {
  RemoteSkill, MarketStatus, InstallResult, LocalSkill,
  IdeSkill, Overview, LinkTarget, DownloadTask
} from "./types";
import { useIdeConfig } from "./useIdeConfig";
import { useMarketConfig } from "./useMarketConfig";

function isSafeRelativePath(input: string): boolean {
  const trimmed = input.trim();
  if (!trimmed) return false;
  if (trimmed.startsWith("/") || /^[A-Za-z]:/i.test(trimmed) || trimmed.startsWith("\\")) {
    return false;
  }
  const parts = trimmed.split(/[\\/]+/);
  return parts.every((part) => part !== ".." && part !== "");
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

  // Local Skills
  const localSkills = ref<LocalSkill[]>([]);
  const ideSkills = ref<IdeSkill[]>([]);
  const localLoading = ref(false);

  // Download Queue
  const downloadQueue = ref<DownloadTask[]>([]);
  let isProcessingQueue = false;

  // Timer tracking for cleanup
  const timers: number[] = [];

  // Cleanup on unmount
  onUnmounted(() => {
    timers.forEach((id) => clearTimeout(id));
  });

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

  const {
    marketConfigs,
    enabledMarkets,
    marketStatuses,
    loadMarketConfigs,
    saveMarketConfigs
  } = useMarketConfig();

  const {
    ideOptions,
    selectedIdeFilter,
    customIdeName,
    customIdeDir,
    customIdeOptions,
    refreshIdeOptions,
    addCustomIde: doAddCustomIde,
    removeCustomIde,
    loadLastInstallTargets,
    saveLastInstallTargets
  } = useIdeConfig();

  function addCustomIde() {
    const success = doAddCustomIde(t, (msg: string) => {
      toast.error(msg);
    });
    if (success) {
      void scanLocalSkills();
    }
  }

  const filteredIdeSkills = computed(() =>
    ideSkills.value.filter((skill) => skill.ide === selectedIdeFilter.value)
  );
  async function buildInstallBaseDir(): Promise<string> {
    const home = await homeDir();
    return join(home, ".skills-manager/skills");
  }

  async function buildLinkTargets(targetLabel: string): Promise<LinkTarget[]> {
    const home = await homeDir();
    const target = ideOptions.value.find((option) => option.label === targetLabel);
    if (!target || !isSafeRelativePath(target.globalDir)) return [];
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

  function addToDownloadQueue(skill: RemoteSkill) {
    // Check if already in queue
    if (downloadQueue.value.some(t => t.id === skill.id)) {
      return;
    }
    downloadQueue.value.push({
      id: skill.id,
      name: skill.name,
      sourceUrl: skill.sourceUrl,
      status: 'pending'
    });
    processQueue();
  }

  async function processQueue() {
    if (isProcessingQueue) return;
    isProcessingQueue = true;

    while (true) {
      const task = downloadQueue.value.find(t => t.status === 'pending');
      if (!task) break;

      task.status = 'downloading';
      try {
        const installBaseDir = await buildInstallBaseDir();
        await invoke("download_marketplace_skill", {
          request: {
            sourceUrl: task.sourceUrl,
            skillName: task.name,
            installBaseDir
          }
        });
        task.status = 'done';
        // Remove completed task after a short delay
        const timerId = window.setTimeout(() => {
          downloadQueue.value = downloadQueue.value.filter(t => t.id !== task.id);
          void scanLocalSkills(); // Properly handle async
          // Clean up timer to prevent memory leaks
          const index = timers.indexOf(timerId);
          if (index > -1) timers.splice(index, 1);
        }, 1500);
        timers.push(timerId);
      } catch (err) {
        task.status = 'error';
        task.error = err instanceof Error ? err.message : String(err);
      }
    }

    isProcessingQueue = false;
  }

  function removeFromQueue(taskId: string) {
    downloadQueue.value = downloadQueue.value.filter(t => t.id !== taskId);
  }

  function retryDownload(taskId: string) {
    const task = downloadQueue.value.find(t => t.id === taskId);
    if (task && task.status === 'error') {
      task.status = 'pending';
      task.error = undefined;
      processQueue();
    }
  }

  // Keep original downloadSkill for backward compatibility (called by updateSkill)
  async function downloadSkill(skill: RemoteSkill) {
    addToDownloadQueue(skill);
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
    downloadQueue,

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
    importLocalSkill,
    addToDownloadQueue,
    removeFromQueue,
    retryDownload
  };
}
