import { ref, computed } from "vue";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getName, getVersion } from "@tauri-apps/api/app";

// Update state
const checking = ref(false);
const updateAvailable = ref(false);
const latestVersion = ref("");
const downloading = ref(false);
const downloadProgress = ref(0);
const downloaded = ref(false);
const upToDate = ref(false);
const error = ref<string | null>(null);

// App info
const appName = ref("Skills Manager");
const currentVersion = ref("");

// Store the update object for later use
let updateObj: Awaited<ReturnType<typeof check>> | null = null;

// Track if we've checked on startup
let startupCheckDone = false;

export function useUpdateStore() {
  // Load app info
  const loadAppInfo = async () => {
    try {
      appName.value = await getName();
      currentVersion.value = await getVersion();
    } catch {
      appName.value = "Skills Manager";
      currentVersion.value = "0.3.1";
    }
  };

  // Check for updates
  const checkUpdate = async () => {
    if (checking.value) return null;

    checking.value = true;
    updateAvailable.value = false;
    upToDate.value = false;
    downloaded.value = false;
    error.value = null;

    try {
      updateObj = await check();
      if (updateObj) {
        latestVersion.value = updateObj.version;
        updateAvailable.value = true;
      } else {
        upToDate.value = true;
      }
      return updateObj;
    } catch (e) {
      console.error("Update check failed", e);
      error.value = e instanceof Error ? e.message : "Update check failed";
      return null;
    } finally {
      checking.value = false;
    }
  };

  // Check for updates on startup (silent, no UI feedback unless update available)
  const checkOnStartup = async () => {
    if (startupCheckDone) return;
    startupCheckDone = true;

    await loadAppInfo();

    try {
      updateObj = await check();
      if (updateObj) {
        latestVersion.value = updateObj.version;
        updateAvailable.value = true;
      }
    } catch (e) {
      // Silent fail on startup
      console.error("Startup update check failed", e);
    }
  };

  // Download and install update
  const downloadUpdate = async () => {
    if (!updateObj || downloading.value) return;

    downloading.value = true;
    downloadProgress.value = 0;

    try {
      await updateObj.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            downloadProgress.value = 0;
            break;
          case "Progress":
            // Progress event only provides chunkLength, not contentLength
            // So we show indeterminate progress with animation instead
            downloadProgress.value = Math.min(downloadProgress.value + 5, 90);
            break;
          case "Finished":
            downloadProgress.value = 100;
            downloaded.value = true;
            downloading.value = false;
            break;
        }
      });
    } catch (e) {
      console.error("Download failed", e);
      error.value = e instanceof Error ? e.message : "Download failed";
      downloading.value = false;
    }
  };

  // Install and restart
  const installAndRestart = async () => {
    try {
      await relaunch();
    } catch (e) {
      console.error("Restart failed", e);
      error.value = e instanceof Error ? e.message : "Restart failed";
    }
  };

  // Reset state (e.g., when navigating away)
  const resetState = () => {
    upToDate.value = false;
    error.value = null;
  };

  return {
    // State
    checking,
    updateAvailable,
    latestVersion,
    downloading,
    downloadProgress,
    downloaded,
    upToDate,
    error,
    appName,
    currentVersion,

    // Computed
    hasUpdate: computed(() => updateAvailable.value && !downloaded.value),

    // Actions
    loadAppInfo,
    checkUpdate,
    checkOnStartup,
    downloadUpdate,
    installAndRestart,
    resetState,
  };
}
