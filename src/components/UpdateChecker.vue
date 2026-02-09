<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const updateAvailable = ref(false);
const latestVersion = ref('');
const updating = ref(false);

let update: Awaited<ReturnType<typeof check>> | null = null;

async function checkUpdate() {
  try {
    update = await check();
    if (update) {
      latestVersion.value = update.version;
      updateAvailable.value = true;
    }
  } catch (e) {
    console.error('Update check failed', e);
  }
}

async function startUpdate() {
  if (update) {
    try {
      updating.value = true;
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      console.error('Update install failed', e);
      updating.value = false;
    }
  }
}

onMounted(() => {
  checkUpdate();
});
</script>

<template>
  <div v-if="updateAvailable" class="update-banner">
    <span>
      {{ t('update.available', { version: latestVersion }) }}
    </span>
    <button @click="startUpdate" :disabled="updating">
      {{ updating ? t('market.updating') : t('update.install') }}
    </button>
    <button class="close" @click="updateAvailable = false">×</button>
  </div>
</template>

<style scoped>
.update-banner {
  position: fixed;
  bottom: 20px;
  right: 20px;
  background: var(--surface-1);
  border: 1px solid var(--primary);
  padding: 12px 16px;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.2);
  display: flex;
  align-items: center;
  gap: 12px;
  z-index: 9999;
  animation: slide-up 0.3s ease-out;
}

.update-banner span {
  font-size: 0.9em;
  font-weight: 500;
}

.update-banner button {
  background: var(--primary);
  color: white;
  border: none;
  padding: 6px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.85em;
  font-weight: 600;
  transition: background 0.2s;
}

.update-banner button:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.update-banner button:hover:not(:disabled) {
  filter: brightness(1.1);
}

.update-banner button.close {
  background: transparent;
  color: var(--text-2);
  padding: 0 4px;
  font-size: 1.2em;
  margin-left: 4px;
  line-height: 1;
}

.update-banner button.close:hover {
  color: var(--text-1);
  background: transparent;
}

@keyframes slide-up {
  from { transform: translateY(20px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
