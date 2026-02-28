import { onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { homeDir, join } from "@tauri-apps/api/path";
import type { RemoteSkill, DownloadTask } from "./types";

export type DownloadQueueCallbacks = {
  onDownloadComplete?: () => void;
};

export function useDownloadQueue(callbacks: DownloadQueueCallbacks = {}) {

  // Download Queue
  const downloadQueue = ref<DownloadTask[]>([]);
  let isProcessingQueue = false;

  // Timer tracking for cleanup
  const timers: number[] = [];

  // Cleanup on unmount
  onUnmounted(() => {
    timers.forEach((id) => clearTimeout(id));
  });

  async function buildInstallBaseDir(): Promise<string> {
    const home = await homeDir();
    return join(home, ".skills-manager/skills");
  }

  function addToDownloadQueue(skill: RemoteSkill) {
    // Check if already in queue
    if (downloadQueue.value.some((t) => t.id === skill.id)) {
      return;
    }
    downloadQueue.value.push({
      id: skill.id,
      name: skill.name,
      sourceUrl: skill.sourceUrl,
      status: "pending",
    });
    processQueue();
  }

  async function processQueue() {
    if (isProcessingQueue) return;
    isProcessingQueue = true;

    while (true) {
      const task = downloadQueue.value.find((t) => t.status === "pending");
      if (!task) break;

      task.status = "downloading";
      try {
        const installBaseDir = await buildInstallBaseDir();
        await invoke("download_marketplace_skill", {
          request: {
            sourceUrl: task.sourceUrl,
            skillName: task.name,
            installBaseDir,
          },
        });
        task.status = "done";
        // Remove completed task after a short delay
        const timerId = window.setTimeout(() => {
          downloadQueue.value = downloadQueue.value.filter((t) => t.id !== task.id);
          callbacks.onDownloadComplete?.();
          // Clean up timer to prevent memory leaks
          const index = timers.indexOf(timerId);
          if (index > -1) timers.splice(index, 1);
        }, 1500);
        timers.push(timerId);
      } catch (err) {
        task.status = "error";
        task.error = err instanceof Error ? err.message : String(err);
        // 在 error 分支也清理 timer，防止内存泄漏
        const timerId = window.setTimeout(() => {
          downloadQueue.value = downloadQueue.value.filter((t) => t.id !== task.id);
          const index = timers.indexOf(timerId);
          if (index > -1) timers.splice(index, 1);
        }, 3000);
        timers.push(timerId);
      }
    }

    isProcessingQueue = false;
  }

  function removeFromQueue(taskId: string) {
    downloadQueue.value = downloadQueue.value.filter((t) => t.id !== taskId);
  }

  function retryDownload(taskId: string) {
    const task = downloadQueue.value.find((t) => t.id === taskId);
    if (task && task.status === "error") {
      task.status = "pending";
      task.error = undefined;
      processQueue();
    }
  }

  return {
    downloadQueue,
    addToDownloadQueue,
    processQueue,
    removeFromQueue,
    retryDownload,
  };
}
