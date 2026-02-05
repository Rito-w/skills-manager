import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { homeDir, join } from "@tauri-apps/api/path";

export type RemoteSkill = {
  id: string;
  name: string;
  namespace: string;
  sourceUrl: string;
  description: string;
  author: string;
  installs: number;
  stars: number;
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
  { id: "qoder", label: "Qoder", globalDir: ".qoder/skills" },
  { id: "trae", label: "Trae", globalDir: ".trae/skills" },
  { id: "vscode", label: "VSCode", globalDir: ".github/skills" },
  { id: "windsurf", label: "Windsurf", globalDir: ".windsurf/skills" }
];

const ideKey = "skillsManager.ideOptions";
const installTargetKey = "skillsManager.lastInstallTargets";

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
  const cacheTtlMs = 10 * 60 * 1000;
  const searchCache = new Map<
    string,
    { timestamp: number; data: { skills: RemoteSkill[]; total: number; limit: number; offset: number } }
  >();
  const activeTab = ref<"local" | "market" | "ide">("local");

  const query = ref("");
  const results = ref<RemoteSkill[]>([]);
  const total = ref(0);
  const limit = ref(20);
  const offset = ref(0);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const installingId = ref<string | null>(null);
  const updatingId = ref<string | null>(null);
  const installMessageRef = ref<string | null>(null);
  const messageTimer = ref<number | null>(null);

  const localSkills = ref<LocalSkill[]>([]);
  const ideSkills = ref<IdeSkill[]>([]);
  const localLoading = ref(false);
  const localError = ref<string | null>(null);

  const ideOptions = ref<IdeOption[]>([]);
  const selectedIdeFilter = ref("Antigravity");
  const customIdeName = ref("");
  const customIdeDir = ref("");

  const showInstallModal = ref(false);
  const installTargetSkill = ref<LocalSkill | null>(null);
  const installTargetIde = ref<string[]>([]);
  const installError = ref<string | null>(null);

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

  const setInstallMessage = (message: string | null) => {
    installMessageRef.value = message;
    if (messageTimer.value) {
      window.clearTimeout(messageTimer.value);
      messageTimer.value = null;
    }
    if (message) {
      messageTimer.value = window.setTimeout(() => {
        installMessageRef.value = null;
        messageTimer.value = null;
      }, 4000);
    }
  };

  const filteredIdeSkills = computed(() =>
    ideSkills.value.filter((skill) => skill.ide === selectedIdeFilter.value)
  );

  const customIdeOptions = computed(() =>
    ideOptions.value.filter((item) => item.id.startsWith("custom-"))
  );

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
      localError.value = t("errors.fillIde");
      return;
    }
    const normalizedName = name.toLowerCase();
    if (ideOptions.value.some((item) => item.label.toLowerCase() === normalizedName)) {
      localError.value = t("errors.ideExists");
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
    localError.value = null;
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
    error.value = null;
    setInstallMessage(null);

    const nextOffset = reset ? 0 : offset.value + limit.value;
    const cacheKey = `${query.value.trim().toLowerCase()}|${limit.value}`;

    if (reset && !force) {
      const cached = searchCache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < cacheTtlMs) {
        results.value = cached.data.skills;
        total.value = cached.data.total;
        offset.value = cached.data.offset;
        loading.value = false;
        return;
      }
    }

    try {
      const response = await invoke("search_marketplace", {
        query: query.value,
        limit: limit.value,
        offset: nextOffset
      });
      const data = response as { skills: RemoteSkill[]; total: number; limit: number; offset: number };

      if (reset) {
        results.value = data.skills;
      } else {
        results.value = [...results.value, ...data.skills];
      }

      total.value = data.total;
      offset.value = data.offset;

      if (reset) {
        searchCache.set(cacheKey, { timestamp: Date.now(), data });
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : t("errors.searchFailed");
    } finally {
      loading.value = false;
    }
  }

  async function downloadSkill(skill: RemoteSkill) {
    if (installingId.value) return;

    installingId.value = skill.id;
    error.value = null;
    setInstallMessage(null);
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

      setInstallMessage(t("messages.downloaded", { path: result.installedPath }));
      await scanLocalSkills();
    } catch (err) {
      error.value = err instanceof Error ? err.message : t("errors.downloadFailed");
    } finally {
      installingId.value = null;
      busy.value = false;
      busyText.value = "";
    }
  }

  async function updateSkill(skill: RemoteSkill) {
    if (updatingId.value) return;

    updatingId.value = skill.id;
    error.value = null;
    setInstallMessage(null);
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

      setInstallMessage(t("messages.updated", { path: result.installedPath }));
      await scanLocalSkills();
    } catch (err) {
      error.value = err instanceof Error ? err.message : t("errors.updateFailed");
    } finally {
      updatingId.value = null;
      busy.value = false;
      busyText.value = "";
    }
  }

  async function scanLocalSkills() {
    if (localLoading.value) return;
    localLoading.value = true;
    localError.value = null;

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
      localError.value = err instanceof Error ? err.message : t("errors.scanFailed");
    } finally {
      localLoading.value = false;
    }
  }

  async function linkSkillInternal(skill: LocalSkill, ideLabel: string, skipScan = false) {
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
    setInstallMessage(t("messages.handled", { linked: linkedCount, skipped: skippedCount }));
    if (!skipScan) {
      await scanLocalSkills();
    }
  }

  function openInstallModal(skill: LocalSkill) {
    installTargetSkill.value = skill;
    const lastTargets = loadLastInstallTargets();
    const available = new Set(ideOptions.value.map((item) => item.label));
    const nextTargets = lastTargets.filter((label) => available.has(label));
    installTargetIde.value = nextTargets;
    installError.value = null;
    showInstallModal.value = true;
  }

  function updateInstallTargetIde(next: string[]) {
    installTargetIde.value = next;
    saveLastInstallTargets(next);
    if (next.length > 0) {
      installError.value = null;
    }
  }

  async function confirmInstallToIde() {
    if (!installTargetSkill.value || installTargetIde.value.length === 0) {
      installError.value = t("errors.selectAtLeastOne");
      return;
    }
    if (installingId.value) return;
    installingId.value = installTargetSkill.value.id;
    localError.value = null;
    installError.value = null;
    setInstallMessage(null);
    busy.value = true;
    busyText.value = t("messages.installing");

    try {
      for (const label of installTargetIde.value) {
        await linkSkillInternal(installTargetSkill.value, label, true);
      }
      await scanLocalSkills();
      showInstallModal.value = false;
      installTargetSkill.value = null;
    } catch (err) {
      localError.value = err instanceof Error ? err.message : t("errors.installFailed");
    } finally {
      installingId.value = null;
      busy.value = false;
      busyText.value = "";
    }
  }

  function closeInstallModal() {
    showInstallModal.value = false;
    installTargetSkill.value = null;
    installError.value = null;
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
      setInstallMessage(message);
      await scanLocalSkills();
    } catch (err) {
      localError.value = err instanceof Error ? err.message : t("errors.uninstallFailed");
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

  onMounted(() => {
    refreshIdeOptions();
    void searchMarketplace(true);
    void scanLocalSkills();
  });

  return {
    activeTab,
    query,
    results,
    total,
    limit,
    offset,
    loading,
    error,
    installingId,
    updatingId,
    installMessage: installMessageRef,
    localSkills,
    ideSkills,
    localLoading,
    localError,
    ideOptions,
    selectedIdeFilter,
    customIdeName,
    customIdeDir,
    showInstallModal,
    installTargetIde,
    installError,
    showUninstallModal,
    uninstallTargetName,
    busy,
    busyText,
    hasMore,
    localSkillNameSet,
    filteredIdeSkills,
    customIdeOptions,
    refreshIdeOptions,
    addCustomIde,
    removeCustomIde,
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
    cancelUninstall
  };
}
