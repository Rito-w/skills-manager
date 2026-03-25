import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { dirname, homeDir, join } from "@tauri-apps/api/path";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useToast } from "./useToast";
import type {
  RemoteSkill, MarketStatus, InstallResult, LocalSkill,
  IdeSkill, Overview, LinkTarget, DownloadTask, ProjectConfig
} from "./types";
import { useIdeConfig } from "./useIdeConfig";
import { useMarketConfig } from "./useMarketConfig";
import { isSafeRelativePath, getErrorMessage, isSafeAbsolutePath } from "./utils";

export function useSkillsManager() {
  const { t } = useI18n();
  const toast = useToast();
  const cacheTtlMs = 10 * 60 * 1000;
  const searchCache = new Map<
    string,
    { timestamp: number; data: { skills: RemoteSkill[]; total: number; limit: number; offset: number; marketStatuses: MarketStatus[] } }
  >();
  const activeTab = ref<"local" | "market" | "ide" | "projects" | "settings">("local");

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
  const installTargetSkills = ref<LocalSkill[]>([]);
  const installTargetIde = ref<string[]>([]);

  const showUninstallModal = ref(false);
  const uninstallTargetPath = ref("");
  const uninstallTargetName = ref("");
  const uninstallTargetPaths = ref<string[]>([]);
  const uninstallMode = ref<"ide" | "local">("ide");

  const busy = ref(false);
  const busyText = ref("");
  const recentTaskStatus = ref<Record<string, "download" | "update">>({});

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

  function sanitizeExportFileName(name: string): string {
    const sanitized = name.trim().replace(/[<>:"/\\|?*\x00-\x1F]/g, "-").replace(/\s+/g, "-");
    return sanitized || "skill";
  }

  function buildExportDefaultName(skills: LocalSkill[]): string {
    if (skills.length === 1) {
      return `${sanitizeExportFileName(skills[0].name)}.zip`;
    }
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    return `skills-export-${timestamp}.zip`;
  }

  async function buildLinkTargets(targetLabel: string): Promise<LinkTarget[]> {
    const target = ideOptions.value.find((option) => option.label === targetLabel);
    if (!target) return [];

    const dir = target.globalDir;

    // Absolute path: use directly
    if (isSafeAbsolutePath(dir)) {
      return [{ name: target.label, path: dir }];
    }

    // Relative path: join with home directory
    if (!isSafeRelativePath(dir)) return [];

    const home = await homeDir();
    return [
      {
        name: target.label,
        path: await join(home, dir)
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
      toast.error(getErrorMessage(err, t("errors.searchFailed")));
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

  function addToDownloadQueue(skill: RemoteSkill, action: "download" | "update" = "download") {
    // Check if already in queue
    if (downloadQueue.value.some(t => t.id === skill.id)) {
      return;
    }
    downloadQueue.value.push({
      id: skill.id,
      name: skill.name,
      sourceUrl: skill.sourceUrl,
      action,
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
        const command = task.action === "update"
          ? "update_marketplace_skill"
          : "download_marketplace_skill";

        await invoke(command, {
          request: {
            sourceUrl: task.sourceUrl,
            skillName: task.name,
            installBaseDir
          }
        });
        task.status = 'done';
        recentTaskStatus.value = {
          ...recentTaskStatus.value,
          [task.id]: task.action
        };
        toast.success(
          task.action === "update"
            ? t("messages.updated", { path: task.name })
            : t("messages.downloaded", { path: task.name })
        );
        // Remove completed task after a short delay
        const timerId = window.setTimeout(() => {
          downloadQueue.value = downloadQueue.value.filter(t => t.id !== task.id);
          const nextStatus = { ...recentTaskStatus.value };
          delete nextStatus[task.id];
          recentTaskStatus.value = nextStatus;
          void scanLocalSkills(); // Properly handle async
          // Clean up timer to prevent memory leaks
          const index = timers.indexOf(timerId);
          if (index > -1) timers.splice(index, 1);
        }, 2500);
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

  // Keep original downloadSkill for backward compatibility
  async function downloadSkill(skill: RemoteSkill) {
    addToDownloadQueue(skill, "download");
  }

  async function updateSkill(skill: RemoteSkill) {
    addToDownloadQueue(skill, "update");
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
      toast.error(getErrorMessage(err, t("errors.scanFailed")));
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

  function openInstallModal(skill: LocalSkill | LocalSkill[]) {
    installTargetSkills.value = Array.isArray(skill) ? skill : [skill];
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

  async function confirmInstallToIde(installTarget: "ide" | "project", targetIds: string[], projects?: ProjectConfig[]) {
    if (installTarget === "project") {
      // Project installation
      if (!projects || projects.length === 0) {
        toast.error("No projects available");
        showInstallModal.value = false;
        installTargetSkills.value = [];
        return;
      }
      
      if (installTargetSkills.value.length === 0 || targetIds.length === 0) {
        toast.error(t("errors.selectAtLeastOne"));
        return;
      }
      if (installingId.value) return;
      installingId.value = installTargetSkills.value.length === 1 ? installTargetSkills.value[0].id : "__batch__";
      busy.value = true;
      busyText.value = t("messages.installing");

      try {
        let totalLinked = 0;
        let totalSkipped = 0;
        
        // Get selected projects
        const selectedProjects = projects.filter(p => targetIds.includes(p.id));
        
        // Install to project directories
        for (const skill of installTargetSkills.value) {
          for (const project of selectedProjects) {
            for (const ideLabel of project.ideTargets) {
              const result = await linkSkillToProjectInternal(skill, project.path, ideLabel, true, true);
              totalLinked += result.linked.length;
              totalSkipped += result.skipped.length;
            }
          }
        }
        
        toast.success(t("messages.handled", { linked: totalLinked, skipped: totalSkipped }));
        await scanLocalSkills();
        showInstallModal.value = false;
        installTargetSkills.value = [];
      } catch (err) {
        toast.error(getErrorMessage(err, t("errors.installFailed")));
      } finally {
        installingId.value = null;
        busy.value = false;
        busyText.value = "";
      }
      return;
    }
    
    // IDE installation (existing logic)
    if (installTargetSkills.value.length === 0 || targetIds.length === 0) {
      toast.error(t("errors.selectAtLeastOne"));
      return;
    }
    if (installingId.value) return;
    installingId.value = installTargetSkills.value.length === 1 ? installTargetSkills.value[0].id : "__batch__";
    busy.value = true;
    busyText.value = t("messages.installing");

    try {
      let totalLinked = 0;
      let totalSkipped = 0;
      
      // Install to global IDE directories
      for (const skill of installTargetSkills.value) {
        for (const label of targetIds) {
          const result = await linkSkillInternal(skill, label, true, true);
          totalLinked += result.linked.length;
          totalSkipped += result.skipped.length;
        }
      }
      
      toast.success(t("messages.handled", { linked: totalLinked, skipped: totalSkipped }));
      await scanLocalSkills();
      showInstallModal.value = false;
      installTargetSkills.value = [];
    } catch (err) {
      toast.error(getErrorMessage(err, t("errors.installFailed")));
    } finally {
      installingId.value = null;
      busy.value = false;
      busyText.value = "";
    }
  }

  async function linkSkillToProjectInternal(skill: LocalSkill, projectPath: string, ideLabel: string, skipScan = false, suppressToast = false) {
    const linkTargets = await buildProjectLinkTargets(projectPath, ideLabel);
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

  async function buildProjectLinkTargets(projectPath: string, ideLabel: string): Promise<LinkTarget[]> {
    const target = ideOptions.value.find((option) => option.label === ideLabel);
    if (!target) return [];

    const dir = target.globalDir;

    // Build project-specific path
    const skillPath = dir.startsWith("/") ? dir : `${projectPath}/${dir}`;
    
    return [{ name: `${target.label} (${projectPath.split('/').pop()})`, path: skillPath }];
  }

  function closeInstallModal() {
    showInstallModal.value = false;
    installTargetSkills.value = [];
  }

  function openUninstallModal(targetPath: string) {
    uninstallMode.value = "ide";
    uninstallTargetPath.value = targetPath;
    uninstallTargetPaths.value = [targetPath];
    uninstallTargetName.value = targetPath.split(/[\\/]/).pop() || targetPath;
    showUninstallModal.value = true;
  }

  function openUninstallManyModal(paths: string[]) {
    if (paths.length === 0) return;
    uninstallMode.value = "ide";
    uninstallTargetPath.value = "";
    uninstallTargetPaths.value = paths;
    uninstallTargetName.value = t("ide.uninstallSelectedCount", { count: paths.length });
    showUninstallModal.value = true;
  }

  function openDeleteLocalModal(targets: LocalSkill[]) {
    uninstallMode.value = "local";
    uninstallTargetPath.value = "";
    uninstallTargetPaths.value = targets.map((skill) => skill.path);
    uninstallTargetName.value =
      targets.length === 1 ? targets[0].name : t("local.deleteSelectedCount", { count: targets.length });
    showUninstallModal.value = true;
  }

  async function confirmUninstall() {
    busy.value = true;
    busyText.value = uninstallMode.value === "local" ? t("messages.deleting") : t("messages.uninstalling");
    try {
      if (uninstallMode.value === "local") {
        const message = ((await invoke("delete_local_skills", {
          request: {
            targetPaths: uninstallTargetPaths.value
          }
        })) as string);
        toast.success(message);
      } else {
        // IDE mode: uninstall each path
        let successCount = 0;
        let failCount = 0;
        for (const targetPath of uninstallTargetPaths.value) {
          try {
            await invoke("uninstall_skill", {
              request: {
                targetPath,
                projectDir: null,
                ideDirs: ideOptions.value.map((item) => ({
                  label: item.label,
                  relativeDir: item.globalDir
                }))
              }
            });
            successCount++;
          } catch {
            failCount++;
          }
        }
        if (successCount > 0 && failCount === 0) {
          toast.success(t("messages.uninstalledCount", { count: successCount }));
        } else if (successCount > 0 && failCount > 0) {
          toast.success(t("messages.uninstalledPartial", { success: successCount, failed: failCount }));
        } else {
          toast.error(t("errors.uninstallFailed"));
        }
      }
      await scanLocalSkills();
    } catch (err) {
      toast.error(
        getErrorMessage(
          err,
          uninstallMode.value === "local" ? t("errors.deleteFailed") : t("errors.uninstallFailed")
        )
      );
    } finally {
      showUninstallModal.value = false;
      uninstallTargetPath.value = "";
      uninstallTargetName.value = "";
      uninstallTargetPaths.value = [];
      busy.value = false;
      busyText.value = "";
    }
  }

  function cancelUninstall() {
    showUninstallModal.value = false;
    uninstallTargetPath.value = "";
    uninstallTargetName.value = "";
    uninstallTargetPaths.value = [];
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
      toast.error(getErrorMessage(err, t("errors.importFailed")));
    } finally {
      busy.value = false;
      busyText.value = "";
    }
  }

  async function exportLocalSkills(skills: LocalSkill[]) {
    if (skills.length === 0) return;

    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const defaultPath = buildExportDefaultName(skills);
      const exportPath = await save({
        title: t("local.selectExportPath"),
        defaultPath,
        filters: [{ name: "ZIP Archive", extensions: ["zip"] }]
      });

      if (!exportPath) return;

      busy.value = true;
      busyText.value = t("messages.exporting");

      const normalizedExportPath = exportPath.toLowerCase().endsWith(".zip")
        ? exportPath
        : `${exportPath}.zip`;

      const result = (await invoke("export_local_skills", {
        request: {
          targetPaths: skills.map((skill) => skill.path),
          exportPath: normalizedExportPath
        }
      })) as string;

      toast.success(t("messages.exported", { path: result }));
    } catch (err) {
      toast.error(getErrorMessage(err, t("errors.exportFailed")));
    } finally {
      busy.value = false;
      busyText.value = "";
    }
  }

  async function openSkillDirectory(path: string) {
    try {
      await revealItemInDir(path);
    } catch (err) {
      const message = getErrorMessage(err, t("errors.openDirFailed"));
      if (message.includes("os error 2") || message.toLowerCase().includes("cannot find the file")) {
        try {
          await revealItemInDir(await dirname(path));
          toast.error(t("errors.openDirFailed") + ": " + path);
          return;
        } catch {
          // Fall through to the original error below.
        }
      }
      toast.error(message);
    }
  }

  async function adoptIdeSkill(skill: IdeSkill) {
    busy.value = true;
    busyText.value = t("messages.adopting");
    try {
      const message = (await invoke("adopt_ide_skill", {
        request: {
          targetPath: skill.path,
          ideLabel: skill.ide
        }
      })) as string;
      toast.success(message);
      await scanLocalSkills();
    } catch (err) {
      toast.error(getErrorMessage(err, t("errors.adoptFailed")));
    } finally {
      busy.value = false;
      busyText.value = "";
    }
  }

  async function adoptManyIdeSkills(skills: IdeSkill[]) {
    if (skills.length === 0) return;
    busy.value = true;
    busyText.value = t("messages.adopting");
    let successCount = 0;
    let failCount = 0;
    try {
      for (const skill of skills) {
        try {
          await invoke("adopt_ide_skill", {
            request: {
              targetPath: skill.path,
              ideLabel: skill.ide
            }
          });
          successCount++;
        } catch {
          failCount++;
        }
      }
      if (successCount > 0 && failCount === 0) {
        toast.success(t("messages.adoptedCount", { count: successCount }));
      } else if (successCount > 0 && failCount > 0) {
        toast.success(t("messages.adoptedPartial", { success: successCount, failed: failCount }));
      } else {
        toast.error(t("errors.adoptFailed"));
      }
      await scanLocalSkills();
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
    uninstallMode,
    recentTaskStatus,

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
    openUninstallManyModal,
    openDeleteLocalModal,
    confirmUninstall,
    cancelUninstall,
    importLocalSkill,
    exportLocalSkills,
    openSkillDirectory,
    adoptIdeSkill,
    adoptManyIdeSkills,
    addToDownloadQueue,
    removeFromQueue,
    retryDownload
  };
}
