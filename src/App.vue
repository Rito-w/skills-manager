<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { i18n, supportedLocales, type SupportedLocale } from "./i18n";
import { useSkillsManager } from "./composables/useSkillsManager";
import { useUpdateStore } from "./composables/useUpdateStore";
import MarketPanel from "./components/MarketPanel.vue";
import LocalPanel from "./components/LocalPanel.vue";
import IdePanel from "./components/IdePanel.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import InstallModal from "./components/InstallModal.vue";
import UninstallModal from "./components/UninstallModal.vue";
import LoadingOverlay from "./components/LoadingOverlay.vue";
import Toast from "./components/Toast.vue";

const { t } = useI18n();

const localeKey = "skillsManager.locale";
const themeKey = "skillsManager.theme";

const theme = ref<"light" | "dark">("light");
const locale = ref<SupportedLocale>("zh-CN");

const applyTheme = (next: "light" | "dark") => {
  document.documentElement.setAttribute("data-theme", next);
};

const loadLocale = (): SupportedLocale => {
  const stored = localStorage.getItem(localeKey) as SupportedLocale | null;
  if (stored && supportedLocales.includes(stored)) return stored;
  const browser = navigator.language.startsWith("zh") ? "zh-CN" : "en-US";
  return browser as SupportedLocale;
};

const loadTheme = (): "light" | "dark" => {
  const stored = localStorage.getItem(themeKey);
  if (stored === "dark" || stored === "light") return stored;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
};

onMounted(() => {
  locale.value = loadLocale();
  theme.value = loadTheme();
  i18n.global.locale.value = locale.value;
  applyTheme(theme.value);

  // Check for updates on startup
  checkOnStartup();
});

watch(locale, (next) => {
  i18n.global.locale.value = next;
  localStorage.setItem(localeKey, next);
});

watch(theme, (next) => {
  applyTheme(next);
  localStorage.setItem(themeKey, next);
});

const {
  activeTab,
  query,
  results,
  loading,
  installingId,
  updatingId,
  localSkills,
  localLoading,
  ideOptions,
  selectedIdeFilter,
  customIdeName,
  customIdeDir,
  customIdeOptions,
  filteredIdeSkills,
  showInstallModal,
  installTargetIde,
  showUninstallModal,
  uninstallTargetName,
  busy,
  busyText,
  hasMore,
  localSkillNameSet,
  searchMarketplace,
  downloadSkill,
  updateSkill,
  scanLocalSkills,
  openInstallModal,
  updateInstallTargetIde,
  addCustomIde,
  removeCustomIde,
  openUninstallModal,
  confirmInstallToIde,
  closeInstallModal,
  confirmUninstall,
  cancelUninstall,
  importLocalSkill,
  marketConfigs,
  marketStatuses,
  enabledMarkets,
  saveMarketConfigs,
  downloadQueue,
  retryDownload,
  removeFromQueue
} = useSkillsManager();

// Update store for startup check and badge
const { updateAvailable, checkOnStartup } = useUpdateStore();
</script>

<template>
  <div class="app">
    <header class="header">
      <div class="header-spacer" />
      <div class="tabs">
        <button class="tab" :class="{ active: activeTab === 'local' }" @click="activeTab = 'local'">
          {{ t("app.tabs.local") }}
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'market' }"
          @click="activeTab = 'market'"
        >
          {{ t("app.tabs.market") }}
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'ide' }"
          @click="activeTab = 'ide'"
        >
          {{ t("app.tabs.ide") }}
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'settings' }"
          @click="activeTab = 'settings'"
        >
          {{ t("app.tabs.settings") }}
          <span v-if="updateAvailable" class="tab-badge"></span>
        </button>
      </div>
      <div class="header-controls">
        <div class="control">
          <button
            class="icon-toggle"
            type="button"
            :aria-label="t('app.header.language')"
            :title="locale === 'zh-CN' ? '中文' : 'English'"
            @click="locale = locale === 'zh-CN' ? 'en-US' : 'zh-CN'"
          >
            <span class="lang-badge">{{ locale === "zh-CN" ? "EN" : "中" }}</span>
          </button>
        </div>
        <div class="control">
          <button
            class="icon-toggle"
            type="button"
            :aria-label="t('app.header.theme')"
            :title="theme === 'light' ? t('app.header.themeLight') : t('app.header.themeDark')"
            @click="theme = theme === 'light' ? 'dark' : 'light'"
          >
            <svg v-if="theme === 'light'" class="icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M12 4a1 1 0 011 1v1a1 1 0 11-2 0V5a1 1 0 011-1Zm6.36 2.64a1 1 0 010 1.41l-.7.7a1 1 0 11-1.41-1.41l.7-.7a1 1 0 011.41 0ZM20 11a1 1 0 010 2h-1a1 1 0 110-2h1Zm-8 2a3 3 0 100-6 3 3 0 000 6Zm-7 0a1 1 0 010-2H4a1 1 0 110-2h1a1 1 0 110 2H4a1 1 0 010 2Zm1.64-7.95a1 1 0 011.41 0l.7.7a1 1 0 11-1.41 1.41l-.7-.7a1 1 0 010-1.41ZM12 18a1 1 0 011 1v1a1 1 0 11-2 0v-1a1 1 0 011-1Zm7.07-1.07a1 1 0 010 1.41l-.7.7a1 1 0 11-1.41-1.41l.7-.7a1 1 0 011.41 0ZM6.34 16.93a1 1 0 011.41 0l.7.7a1 1 0 11-1.41 1.41l-.7-.7a1 1 0 010-1.41Z"
                fill="currentColor"
              />
            </svg>
            <svg v-else class="icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M21 14.5A8.5 8.5 0 019.5 3a.9.9 0 00-.9.9 9.6 9.6 0 0010.5 10.5.9.9 0 00.9-.9Z"
                fill="currentColor"
              />
            </svg>
          </button>
        </div>
      </div>
    </header>

    <main class="content">
      <template v-if="activeTab === 'local'">
        <LocalPanel
          :local-skills="localSkills"
          :local-loading="localLoading"
          :installing-id="installingId"
          :download-queue="downloadQueue"
          @install="openInstallModal"
          @refresh="scanLocalSkills"
          @import="importLocalSkill"
          @retry-download="retryDownload"
          @remove-from-queue="removeFromQueue"
        />
      </template>

      <template v-else-if="activeTab === 'market'">
        <MarketPanel
          v-model:query="query"
          :loading="loading"
          :results="results"
          :has-more="hasMore"
          :installing-id="installingId"
          :updating-id="updatingId"
          :local-skill-name-set="localSkillNameSet"
          :market-configs="marketConfigs"
          :market-statuses="marketStatuses"
          :enabled-markets="enabledMarkets"
          :download-queue="downloadQueue"
          @search="searchMarketplace(true)"
          @refresh="searchMarketplace(true, true)"
          @loadMore="searchMarketplace(false)"
          @download="downloadSkill"
          @update="updateSkill"
          @saveConfigs="saveMarketConfigs"
        />
      </template>

      <template v-else-if="activeTab === 'ide'">
        <IdePanel
          :ide-options="ideOptions"
          :selected-ide-filter="selectedIdeFilter"
          :custom-ide-name="customIdeName"
          :custom-ide-dir="customIdeDir"
          :custom-ide-options="customIdeOptions"
          :filtered-ide-skills="filteredIdeSkills"
          :local-loading="localLoading"
          @update:selected-ide-filter="selectedIdeFilter = $event"
          @update:custom-ide-name="customIdeName = $event"
          @update:custom-ide-dir="customIdeDir = $event"
          @add-custom-ide="addCustomIde"
          @remove-custom-ide="removeCustomIde"
          @uninstall="openUninstallModal"
        />
      </template>

      <template v-else-if="activeTab === 'settings'">
        <SettingsPanel />
      </template>
    </main>

    <InstallModal
      :visible="showInstallModal"
      :ide-options="ideOptions"
      :selected="installTargetIde"
      @update:selected="updateInstallTargetIde"
      @confirm="confirmInstallToIde"
      @cancel="closeInstallModal"
    />

    <UninstallModal
      :visible="showUninstallModal"
      :target-name="uninstallTargetName"
      @confirm="confirmUninstall"
      @cancel="cancelUninstall"
    />

    <Toast />

    <LoadingOverlay :visible="busy" :text="busyText" />
  </div>
</template>

<style>
/* Global styles moved from App.vue */
:root {
  --color-bg: #f5f5f7;
  --color-text: #1d1d1f;
  --color-muted: #6e6e73;
  --color-panel-bg: #ffffff;
  --color-panel-border: #d2d2d7;
  --color-panel-shadow: rgba(0, 0, 0, 0.04);
  --color-card-bg: #fafafa;
  --color-card-border: #e5e5ea;
  --color-input-bg: #ffffff;
  --color-input-border: #d2d2d7;
  --color-input-focus: #0071e3;
  --color-primary-bg: #0071e3;
  --color-primary-text: #ffffff;
  --color-chip-bg: #e8e8ed;
  --color-chip-border: #d2d2d7;
  --color-chip-text: #1d1d1f;
  --color-tabs-bg: #e8e8ed;
  --color-tab-text: #6e6e73;
  --color-tab-active-bg: #ffffff;
  --color-tab-active-text: #1d1d1f;
  --color-success-bg: #e3f9e5;
  --color-success-border: #b8e6bc;
  --color-success-text: #1e7e34;
  --color-error-bg: #fee2e2;
  --color-error-border: #fecaca;
  --color-error-text: #dc2626;
  --color-warning-bg: #fef3c7;
  --color-warning-border: #fde68a;
  --color-warning-text: #d97706;
  --color-progress-bg: #e5e5ea;
  --color-ghost-border: #d2d2d7;
  --color-ghost-text: #1d1d1f;
  --color-overlay-bg: rgba(0, 0, 0, 0.4);
}

[data-theme="dark"] {
  --color-bg: #1c1c1e;
  --color-text: #f5f5f7;
  --color-muted: #a1a1a6;
  --color-panel-bg: #2c2c2e;
  --color-panel-border: #3a3a3c;
  --color-panel-shadow: rgba(0, 0, 0, 0.3);
  --color-card-bg: #3a3a3c;
  --color-card-border: #48484a;
  --color-input-bg: #2c2c2e;
  --color-input-border: #48484a;
  --color-input-focus: #0a84ff;
  --color-primary-bg: #0a84ff;
  --color-primary-text: #ffffff;
  --color-chip-bg: #3a3a3c;
  --color-chip-border: #48484a;
  --color-chip-text: #f5f5f7;
  --color-tabs-bg: #3a3a3c;
  --color-tab-text: #a1a1a6;
  --color-tab-active-bg: #48484a;
  --color-tab-active-text: #f5f5f7;
  --color-success-bg: #1e3a2f;
  --color-success-border: #2d5a47;
  --color-success-text: #32d74b;
  --color-error-bg: #3d1f1f;
  --color-error-border: #5c3030;
  --color-error-text: #ff453a;
  --color-warning-bg: #3d3a1f;
  --color-warning-border: #5c5830;
  --color-warning-text: #ffd60a;
  --color-progress-bg: #48484a;
  --color-ghost-border: #48484a;
  --color-ghost-text: #f5f5f7;
  --color-overlay-bg: rgba(0, 0, 0, 0.6);
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background: var(--color-bg);
  color: var(--color-text);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

button {
  font-family: inherit;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: var(--color-chip-bg);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--color-muted);
}
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.header {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 12px 20px;
  background: var(--color-panel-bg);
  border-bottom: 1px solid var(--color-panel-border);
  flex-shrink: 0;
}

.header-spacer {
  flex: 1;
  min-width: 120px;
}

.tabs {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--color-tabs-bg);
  border-radius: 10px;
}

.tab {
  position: relative;
  padding: 8px 20px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--color-tab-text);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s ease, color 0.2s ease;
}

.tab:hover {
  color: var(--color-text);
}

.tab.active {
  background: var(--color-tab-active-bg);
  color: var(--color-tab-active-text);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.tab-badge {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 8px;
  height: 8px;
  background: #ff3b30;
  border-radius: 50%;
  border: 2px solid var(--color-tab-active-bg);
}

.tab.active .tab-badge {
  border-color: var(--color-tab-active-bg);
}

.header-controls {
  flex: 1;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  min-width: 120px;
}

.control {
  display: flex;
  align-items: center;
}

.icon-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--color-muted);
  cursor: pointer;
  transition: background 0.2s ease, color 0.2s ease;
}

.icon-toggle:hover {
  background: var(--color-tabs-bg);
  color: var(--color-text);
}

.icon {
  width: 20px;
  height: 20px;
}

.lang-badge {
  font-size: 12px;
  font-weight: 600;
}

.content {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
</style>
