<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { i18n, supportedLocales, type SupportedLocale } from "./i18n";
import { useSkillsManager } from "./composables/useSkillsManager";
import MarketPanel from "./components/MarketPanel.vue";
import LocalPanel from "./components/LocalPanel.vue";
import IdePanel from "./components/IdePanel.vue";
import InstallModal from "./components/InstallModal.vue";
import UninstallModal from "./components/UninstallModal.vue";
import LoadingOverlay from "./components/LoadingOverlay.vue";

const localeKey = "skillsManager.locale";
const themeKey = "skillsManager.theme";

const { t } = useI18n();

const {
  activeTab,
  query,
  results,
  loading,
  error,
  installingId,
  updatingId,
  installMessage,
  localSkills,
  localLoading,
  localError,
  ideOptions,
  selectedIdeFilter,
  customIdeName,
  customIdeDir,
  customIdeOptions,
  filteredIdeSkills,
  showInstallModal,
  installTargetIde,
  installError,
  showUninstallModal,
  uninstallTargetName,
  busy,
  busyText,
  hasMore,
  localSkillNameSet,
  searchMarketplace,
  downloadSkill,
  updateSkill,
  openInstallModal,
  updateInstallTargetIde,
  addCustomIde,
  removeCustomIde,
  openUninstallModal,
  confirmInstallToIde,
  closeInstallModal,
  confirmUninstall,
  cancelUninstall
} = useSkillsManager();

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
});

watch(locale, (next) => {
  i18n.global.locale.value = next;
  localStorage.setItem(localeKey, next);
});

watch(theme, (next) => {
  applyTheme(next);
  localStorage.setItem(themeKey, next);
});
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

    <template v-if="activeTab === 'local'">
      <LocalPanel
        :local-skills="localSkills"
        :local-loading="localLoading"
        :local-error="localError"
        :install-message="installMessage"
        :installing-id="installingId"
        @install="openInstallModal"
      />
    </template>

    <template v-else-if="activeTab === 'market'">
      <MarketPanel
        v-model:query="query"
        :loading="loading"
        :error="error"
        :install-message="installMessage"
        :results="results"
        :has-more="hasMore"
        :installing-id="installingId"
        :updating-id="updatingId"
        :local-skill-name-set="localSkillNameSet"
        @search="searchMarketplace(true)"
        @refresh="searchMarketplace(true, true)"
        @loadMore="searchMarketplace(false)"
        @download="downloadSkill"
        @update="updateSkill"
      />
    </template>

    <template v-else>
      <IdePanel
        :ide-options="ideOptions"
        :selected-ide-filter="selectedIdeFilter"
        :custom-ide-name="customIdeName"
        :custom-ide-dir="customIdeDir"
        :custom-ide-options="customIdeOptions"
        :filtered-ide-skills="filteredIdeSkills"
        :local-error="localError"
        :local-loading="localLoading"
        @update:selected-ide-filter="selectedIdeFilter = $event"
        @update:custom-ide-name="customIdeName = $event"
        @update:custom-ide-dir="customIdeDir = $event"
        @add-custom-ide="addCustomIde"
        @remove-custom-ide="removeCustomIde"
        @uninstall="openUninstallModal"
      />
    </template>

    <InstallModal
      :visible="showInstallModal"
      :ide-options="ideOptions"
      :selected="installTargetIde"
      :error-message="installError"
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

    <LoadingOverlay :visible="busy" :text="busyText" />
  </div>
</template>
