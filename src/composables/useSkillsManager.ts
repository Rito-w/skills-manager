import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { dirname, homeDir, join } from "@tauri-apps/api/path";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useToast } from "./useToast";
import type {
  RemoteSkill, MarketStatus, InstallResult, LocalSkill,
  IdeSkill, Overview, LinkTarget, DownloadTask, ProjectConfig, MarketSortMode
} from "./types";
import { buildProjectLinkTargets } from "./projectTargets";
import { useIdeConfig } from "./useIdeConfig";
import { useMarketConfig } from "./useMarketConfig";
import {
  isSafeRelativePath,
  getErrorMessage,
  isSafeAbsolutePath,
  parseManualSkillSource,
  normalizeSkillName
} from "./utils";

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
  const marketSortMode = ref<MarketSortMode>("default");
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
  const installTargetProjectIds = ref<string[]>([]);

  const showUninstallModal = ref(false);
  const uninstallTargetPath = ref("");
  const uninstallTargetName = ref("");
  const uninstallTargetPaths = ref<string[]>([]);
  const uninstallMode = ref<"ide" | "local">("ide");

  const busy = ref(false);
  const busyText = ref("");
  const recentTaskStatus = ref<Record<string, "download" | "update">>({});

  const hasMore = computed(() => results.value.length < total.value);
  const sortedResults = computed(() => {
    const sortBy = marketSortMode.value === "installs_desc" ? "installs" : "stars";
    return [...results.value].sort((left, right) => {
      const diff = right[sortBy] - left[sortBy];
      if (diff !== 0) return diff;
      return left.name.localeCompare(right.name);
    });
  });
  const localSkillNameSet = computed(() => {
    const set = new Set<string>();
    for (const skill of localSkills.value) {
      const nameKey = normalizeSkillName(skill.name);
      if (nameKey) set.add(nameKey);

      const pathKey = normalizeSkillName(skill.path.split(/[\\/]/).filter(Boolean).pop() ?? "");
      if (pathKey) set.add(pathKey);
    }
    return set;
  });

  const {
    marketConfigs,
    enabledMarkets,
    marketStatuses,
    loadMarketConfigs,
    saveMarketConfigs: persistMarketConfigs
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

  // Make sure we export installTargetProjectIds


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

  async function addManualSkill(sourceUrl: string, customName?: string) {
    const parsed = parseManualSkillSource(sourceUrl);
    if (!parsed) {
      toast.error(t("errors.unsupportedManualUrl"));
      return null;
    }

    const resolvedName = (customName?.trim() || parsed.inferredName || "").trim();
    if (!resolvedName) {
      toast.error(t("errors.manualSkillNameRequired"));
      return null;
    }

    const remoteSkill: RemoteSkill = {
      id: `manual:${parsed.normalizedUrl}`,
      name: resolvedName,
      namespace: "manual",
      sourceUrl: parsed.normalizedUrl,
      description: t("market.manualDescription"),
      author: parsed.kind === "zip" ? t("market.manualSourceLabel") : "",
      installs: 0,
      stars: 0,
      marketId: "manual",
      marketLabel: t("market.manualSourceLabel")
    };

    if (localSkillNameSet.value.has(normalizeSkillName(resolvedName))) {
      await updateSkill(remoteSkill);
      return "update" as const;
    }

    await downloadSkill(remoteSkill);
    return "download" as const;
  }

  function saveMarketConfigs(configs: Record<string, string>, enabled: Record<string, boolean>): void {
    searchCache.clear();
    persistMarketConfigs(configs, enabled);
    void searchMarketplace(true, true);
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

  async function openInstallModal(skill: LocalSkill | LocalSkill[]) {
    installTargetSkills.value = Array.isArray(skill) ? skill : [skill];
    
    if (installTargetSkills.value.length === 1) {
      const singleSkill = installTargetSkills.value[0];
      installTargetIde.value = ideOptions.value
        .filter(ide => singleSkill.usedBy.includes(ide.label))
        .map(ide => ide.label);

      try {
        const raw = localStorage.getItem("skillsManager.projects");
        const projects: ProjectConfig[] = raw ? JSON.parse(raw) : [];
        const installedProjectIds: string[] = [];
        for (const p of projects) {
          const pSkills = await invoke<LocalSkill[]>("scan_project_skills", {
            request: { 
              projectDir: p.path,
              ideDirs: ideOptions.value.map(ide => ({
                label: ide.label,
                relativeDir: ide.projectDir || ide.globalDir
              }))
            }
          });
          if (pSkills.some(s => s.name === singleSkill.name)) {
            installedProjectIds.push(p.id);
          }
        }
        installTargetProjectIds.value = installedProjectIds;
      } catch (err) {
        console.error("Failed to load project skills for modal", err);
        installTargetProjectIds.value = [];
      }
    } else {
      installTargetIde.value = [];
      installTargetProjectIds.value = [];
    }

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
      
      if (installTargetSkills.value.length === 0) {
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
        let totalUninstalled = 0;
        
        for (const skill of installTargetSkills.value) {
          const originalProjectIds = installTargetProjectIds.value;
          const targetsToAdd = targetIds.filter(id => !originalProjectIds.includes(id));
          const targetsToRemove = installTargetSkills.value.length === 1 
            ? originalProjectIds.filter(id => !targetIds.includes(id))
            : [];
            
          for (const projectId of targetsToAdd) {
            const project = projects.find(p => p.id === projectId);
            if (project) {
              const linkTargets: LinkTarget[] = [];
              const targetDirs = new Set<string>();
              
              if (project.ideTargets && project.ideTargets.length > 0) {
                for (const ideLabel of project.ideTargets) {
                  const targetIde = ideOptions.value.find(ide => ide.label === ideLabel);
                  if (targetIde) {
                    const dir = targetIde.projectDir || targetIde.globalDir;
                    if (!targetDirs.has(dir)) {
                      targetDirs.add(dir);
                      linkTargets.push({ name: `Project (${dir})`, path: `${project.path}/${dir}` });
                    }
                  }
                }
              }
              
              if (linkTargets.length === 0) {
                linkTargets.push({ name: `Project (.agents/skills)`, path: `${project.path}/.agents/skills` });
              }

              try {
                const result = (await invoke("link_local_skill", {
                  request: {
                    skillPath: skill.path,
                    skillName: skill.name,
                    linkTargets
                  }
                })) as InstallResult;
                totalLinked += result.linked.length;
                totalSkipped += result.skipped.length;
              } catch (err) {
                console.error(`Failed to link skill to project ${project.name}:`, err);
              }
            }
          }

          for (const projectId of targetsToRemove) {
            const project = projects.find(p => p.id === projectId);
            if (project) {
              try {
                const pSkills = await invoke<LocalSkill[]>("scan_project_skills", {
                  request: { 
                    projectDir: project.path,
                    ideDirs: ideOptions.value.map(ide => ({
                      label: ide.label,
                      relativeDir: ide.projectDir || ide.globalDir
                    }))
                  }
                });
                const installedSkills = pSkills.filter(s => s.name === skill.name);
                for (const installedSkill of installedSkills) {
                  await invoke("uninstall_skill", {
                    request: {
                      targetPath: installedSkill.path,
                      projectDir: project.path,
                      ideDirs: []
                    }
                  });
                  totalUninstalled++;
                }
              } catch (err) {
                console.error(`Failed to uninstall skill from project ${project.name}:`, err);
              }
            }
          }
        }
        
        if (totalLinked > 0 || totalUninstalled > 0) {
          toast.success(t("messages.handledDetailed", { linked: totalLinked, skipped: totalSkipped, uninstalled: totalUninstalled }));
        } else {
          toast.info(t("messages.handledDetailed", { linked: totalLinked, skipped: totalSkipped, uninstalled: totalUninstalled }));
        }
        
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
    
    // IDE installation
    if (installTargetSkills.value.length === 0) {
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
      let totalUninstalled = 0;
      
      // Install to global IDE directories
      for (const skill of installTargetSkills.value) {
        const originalTargets = ideOptions.value
          .filter(ide => skill.usedBy.includes(ide.label))
          .map(ide => ide.label);

        const targetsToAdd = targetIds.filter(t => !originalTargets.includes(t));
        
        // Only perform removals if we are managing a single skill
        const targetsToRemove = installTargetSkills.value.length === 1 
          ? originalTargets.filter(t => !targetIds.includes(t))
          : [];

        // Add
        for (const label of targetsToAdd) {
          const result = await linkSkillInternal(skill, label, true, true);
          totalLinked += result.linked.length;
          totalSkipped += result.skipped.length;
        }

        // Remove
        for (const label of targetsToRemove) {
          const installedSkill = ideSkills.value.find(
            s => s.ide === label && s.name === skill.name
          );
          if (installedSkill) {
            await invoke("uninstall_skill", {
              request: {
                targetPath: installedSkill.path,
                projectDir: null,
                ideDirs: ideOptions.value.map(item => ({
                  label: item.label,
                  relativeDir: item.globalDir
                }))
              }
            });
            totalUninstalled++;
          }
        }
      }
      
      if (totalLinked > 0 || totalUninstalled > 0) {
        toast.success(t("messages.handledDetailed", { linked: totalLinked, skipped: totalSkipped, uninstalled: totalUninstalled }));
      } else {
        toast.info(t("messages.handledDetailed", { linked: totalLinked, skipped: totalSkipped, uninstalled: totalUninstalled }));
      }
      
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

  async function linkSkillToProjectInternal(
    skill: LocalSkill,
    project: ProjectConfig,
    ideLabel: string,
    skipScan = false,
    suppressToast = false
  ) {
    const linkTargets = buildProjectLinkTargets(project, ideLabel, ideOptions.value);
    if (linkTargets.length === 0) {
      throw new Error(`${t("errors.selectValidIde")} (${project.name}: ${ideLabel})`);
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

  function closeInstallModal() {
    showInstallModal.value = false;
    installTargetSkills.value = [];
    installTargetProjectIds.value = [];
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
    installTargetProjectIds,
    showUninstallModal,
    uninstallTargetName,
    busy,
    busyText,
    hasMore,
    sortedResults,
    marketSortMode,
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
    addManualSkill,
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
