<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-opener';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const updateAvailable = ref(false);
const latestVersion = ref('');
const releaseUrl = ref('');

async function checkUpdate() {
  try {
    const current = await getVersion();
    const response = await fetch('https://api.github.com/repos/Rito-w/skills-manager/releases/latest');
    if (!response.ok) return;
    
    const data = await response.json();
    const tagName = data.tag_name; // e.g. "v0.3.0"
    const remoteVersion = tagName.replace(/^v/, '');
    
    if (compareVersions(current, remoteVersion) < 0) {
      latestVersion.value = remoteVersion;
      releaseUrl.value = data.html_url;
      updateAvailable.value = true;
    }
  } catch (e) {
    console.error('Update check failed', e);
  }
}

function compareVersions(a: string, b: string) {
  const pa = a.split('.').map(Number);
  const pb = b.split('.').map(Number);
  for (let i = 0; i < 3; i++) {
    const na = pa[i] || 0;
    const nb = pb[i] || 0;
    if (na > nb) return 1;
    if (nb > na) return -1;
  }
  return 0;
}

function openRelease() {
  if (releaseUrl.value) {
    open(releaseUrl.value);
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
    <button @click="openRelease">{{ t('update.view') }}</button>
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

.update-banner button:hover {
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
