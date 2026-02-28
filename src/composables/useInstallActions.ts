import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./useToast";
import type { LocalSkill, InstallResult } from "./types";

export type InstallActionsCallbacks = {
  onScan: () => Promise<void>;
  onBuildLinkTargets: (ideLabel: string) => Promise<Array<{ name: string; path: string }>>;
  getIdeOptions: () => Array<{ label: string; globalDir: string }>;
};

export function useInstallActions(callbacks: InstallActionsCallbacks) {
  const { t } = useI18n();
  const toast = useToast();

  // Install Modal State
  const showInstallModal = ref(false);
  const installTargetSkill = ref<LocalSkill | null>(null);
  const installTargetIde = ref<string[]>([]);
  const installingId = ref<string | null>(null);

  // Uninstall Modal State
  const showUninstallModal = ref(false);
  const uninstallTargetPath = ref("");
  const uninstallTargetName = ref("");

  // Busy State
  const busy = ref(false);
  const busyText = ref("");

  // LocalStorage helpers
  function loadLastInstallTargets(): string[] {
    try {
      const raw = localStorage.getItem("skillsManager.lastInstallTargets");
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((item) => typeof item === "string");
    } catch {
      return [];
    }
  }

  function saveLastInstallTargets(labels: string[]): void {
    localStorage.setItem("skillsManager.lastInstallTargets", JSON.stringify(labels));
  }

  async function linkSkillInternal(
    skill: LocalSkill,
    ideLabel: string,
    skipScan = false,
    suppressToast = false
  ): Promise<InstallResult> {
    const linkTargets = await callbacks.onBuildLinkTargets(ideLabel);
    if (linkTargets.length === 0) {
      throw new Error(t("errors.selectValidIde"));
    }
    const result = (await invoke("link_local_skill", {
      request: {
        skillPath: skill.path,
        skillName: skill.name,
        linkTargets,
      },
    })) as InstallResult;

    const linkedCount = result.linked.length;
    const skippedCount = result.skipped.length;
    if (!suppressToast) {
      toast.success(t("messages.handled", { linked: linkedCount, skipped: skippedCount }));
    }
    if (!skipScan) {
      await callbacks.onScan();
    }
    return result;
  }

  function openInstallModal(skill: LocalSkill, lastTargets: string[] = []) {
    installTargetSkill.value = skill;
    const ideOptions = callbacks.getIdeOptions();
    const available = new Set(ideOptions.map((item) => item.label));
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
      await callbacks.onScan();
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
      const ideOptions = callbacks.getIdeOptions();
      const message = (await invoke("uninstall_skill", {
        request: {
          targetPath: uninstallTargetPath.value,
          projectDir: null,
          ideDirs: ideOptions.map((item) => ({
            label: item.label,
            relativeDir: item.globalDir,
          })),
        },
      })) as string;
      toast.success(message);
      await callbacks.onScan();
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
        title: t("local.selectSkillDir"),
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
              sourcePath: path,
            },
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

      await callbacks.onScan();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("errors.importFailed"));
    } finally {
      busy.value = false;
      busyText.value = "";
    }
  }

  return {
    // State
    showInstallModal,
    installTargetSkill,
    installTargetIde,
    showUninstallModal,
    uninstallTargetPath,
    uninstallTargetName,
    busy,
    busyText,
    installingId,

    // Actions
    openInstallModal,
    updateInstallTargetIde,
    confirmInstallToIde,
    closeInstallModal,
    openUninstallModal,
    confirmUninstall,
    cancelUninstall,
    importLocalSkill,
    loadLastInstallTargets,
  };
}
