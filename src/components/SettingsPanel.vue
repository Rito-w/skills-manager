<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import { i18n, supportedLocales, type SupportedLocale } from "../i18n";
import { useUpdateStore } from "../composables/useUpdateStore";
import { useToast } from "../composables/useToast";
import { useIdeConfig } from "../composables/useIdeConfig";

const { t } = useI18n();
const toast = useToast();
const { 
  ideOptions, 
  hiddenIdes, 
  toggleIdeVisibility, 
  refreshIdeOptions,
  customIdeName,
  customIdeDir,
  addCustomIde,
  removeCustomIde
} = useIdeConfig();

function handleAddCustomIde() {
  addCustomIde(
    (key: string) => t(key as any),
    (msg: string) => toast.error(msg)
  );
}

// Theme state
type ThemeMode = "light" | "dark" | "system";
const themeKey = "skillsManager.theme";
const localeKey = "skillsManager.locale";
const theme = ref<ThemeMode>("system");
const locale = ref<SupportedLocale>("zh-CN");

// Use shared update store
const {
  appName,
  currentVersion,
  checking,
  updateAvailable,
  latestVersion,
  downloading,
  downloadProgress,
  downloaded,
  upToDate,
  error,
  loadAppInfo,
  checkUpdate,
  downloadUpdate,
  installAndRestart,
  resetState,
} = useUpdateStore();

const handleCheckUpdate = async () => {
  await checkUpdate();
  if (error.value) {
    toast.error(error.value);
  } else if (upToDate.value) {
    toast.info(t("settings.update.upToDate"));
  }
};

const handleDownloadUpdate = async () => {
  await downloadUpdate();
  if (error.value) {
    toast.error(error.value);
  }
};

const handleInstallAndRestart = async () => {
  await installAndRestart();
  if (error.value) {
    toast.error(error.value);
  }
};

// Apply theme to document
const applyTheme = (mode: ThemeMode) => {
  let effectiveTheme: "light" | "dark";
  if (mode === "system") {
    effectiveTheme = window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  } else {
    effectiveTheme = mode;
  }
  document.documentElement.setAttribute("data-theme", effectiveTheme);
};

// Load saved preferences
const loadTheme = (): ThemeMode => {
  const stored = localStorage.getItem(themeKey);
  if (stored === "dark" || stored === "light" || stored === "system") {
    return stored;
  }
  return "system";
};

const loadLocale = (): SupportedLocale => {
  const stored = localStorage.getItem(localeKey) as SupportedLocale | null;
  if (stored && supportedLocales.includes(stored)) return stored;
  const browser = navigator.language.startsWith("zh") ? "zh-CN" : "en-US";
  return browser as SupportedLocale;
};

// Open GitHub
function openGitHub() {
  openUrl("https://github.com/Rito-w/skills-manager");
}

// Watch for theme changes
watch(theme, (next) => {
  applyTheme(next);
  localStorage.setItem(themeKey, next);
});

// Watch for locale changes
watch(locale, (next) => {
  i18n.global.locale.value = next;
  localStorage.setItem(localeKey, next);
});

// Listen for system theme changes
onMounted(async () => {
  // Load app info (may already be loaded by startup check)
  await loadAppInfo();
  refreshIdeOptions();

  // Load preferences
  theme.value = loadTheme();
  locale.value = loadLocale();
  i18n.global.locale.value = locale.value;
  applyTheme(theme.value);

  // Reset upToDate state when entering settings (so we can check again)
  resetState();

  // Listen for system theme changes
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", () => {
      if (theme.value === "system") {
        applyTheme("system");
      }
    });
});
</script>

<template>
  <div class="settings-panel">
    <!-- About Section -->
    <section class="settings-section">
      <h2 class="section-title">{{ t("settings.about.title") }}</h2>
      <div class="about-content">
        <div class="app-info">
          <span class="app-name">{{ appName }}</span>
          <span class="version-badge">v{{ currentVersion }}</span>
        </div>

        <!-- Update status -->
        <div v-if="updateAvailable && !downloaded" class="update-available">
          <span class="update-message">
            {{ t("settings.update.newVersionAvailable", { version: latestVersion }) }}
          </span>
          <button
            v-if="!downloading"
            class="primary"
            @click="handleDownloadUpdate"
          >
            {{ t("settings.update.downloadAndInstall") }}
          </button>
        </div>

        <!-- Downloading progress -->
        <div v-if="downloading" class="downloading">
          <span class="download-status">{{ t("settings.update.downloading") }}</span>
          <div class="progress">
            <div class="progress-bar" :style="{ width: downloadProgress + '%' }"></div>
          </div>
          <span class="progress-text">{{ downloadProgress }}%</span>
        </div>

        <!-- Download complete -->
        <div v-if="downloaded" class="download-complete">
          <span class="complete-message">{{ t("settings.update.installAndRestart") }}</span>
          <button class="primary" @click="handleInstallAndRestart">
            {{ t("settings.update.installAndRestart") }}
          </button>
        </div>

        <!-- Up to date message -->
        <div v-if="upToDate" class="up-to-date">
          <svg class="check-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M20 6L9 17l-5-5" />
          </svg>
          <span>{{ t("settings.update.upToDate") }}</span>
        </div>

        <div class="about-actions">
          <button class="ghost" @click="handleCheckUpdate" :disabled="checking || downloading">
            {{ checking ? t("settings.update.checking") || "..." : t("settings.about.checkUpdate") }}
          </button>
          <button class="ghost" @click="openGitHub">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="currentColor">
              <path
                d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
              />
            </svg>
            {{ t("settings.about.github") }}
          </button>
        </div>
      </div>
    </section>
    <!-- Ides Section -->
    <section class="settings-section">
      <h2 class="section-title">{{ t("settings.ides.title") }}</h2>
      <div class="appearance-content">
        <p class="setting-hint">{{ t("settings.ides.hint") }}</p>
        <div class="ides-grid">
          <label 
            v-for="ide in ideOptions" 
            :key="ide.id" 
            class="ide-checkbox"
          >
            <input 
              type="checkbox" 
              :checked="!hiddenIdes.includes(ide.id)"
              @change="toggleIdeVisibility(ide.id)"
            />
            <span class="ide-label">{{ ide.label }}</span>
            <button
              v-if="ide.id.startsWith('custom-')"
              class="ide-delete-btn"
              @click.stop.prevent="removeCustomIde(ide.label)"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </label>
        </div>
        <div style="margin-top: 16px;">
          <h3 class="section-title" style="margin-bottom: 12px; font-size: 14px;">{{ t("ide.addHint") }}</h3>
          <div style="display: flex; gap: 10px; align-items: center; max-width: 600px;">
            <input
              v-model="customIdeName"
              class="ide-input"
              :placeholder="t('ide.namePlaceholder')"
              style="flex: 1; padding: 8px 12px; border: 1px solid var(--color-input-border); border-radius: 8px; background: var(--color-input-bg); color: var(--color-text);"
            />
            <input
              v-model="customIdeDir"
              class="ide-input"
              :placeholder="t('ide.dirPlaceholder')"
              style="flex: 2; padding: 8px 12px; border: 1px solid var(--color-input-border); border-radius: 8px; background: var(--color-input-bg); color: var(--color-text);"
            />
            <button class="primary" @click="handleAddCustomIde">{{ t("ide.addButton") }}</button>
          </div>
        </div>
      </div>
    </section>
    <!-- Appearance Section -->
    <section class="settings-section">
      <h2 class="section-title">{{ t("settings.appearance.title") }}</h2>
      <div class="appearance-content">
        <!-- Theme -->
        <div class="setting-row">
          <label class="setting-label">{{ t("settings.appearance.theme") }}</label>
          <div class="theme-options">
            <button
              class="theme-btn"
              :class="{ active: theme === 'light' }"
              @click="theme = 'light'"
            >
              <svg class="theme-icon" viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M12 4a1 1 0 011 1v1a1 1 0 11-2 0V5a1 1 0 011-1Zm6.36 2.64a1 1 0 010 1.41l-.7.7a1 1 0 11-1.41-1.41l.7-.7a1 1 0 011.41 0ZM20 11a1 1 0 010 2h-1a1 1 0 110-2h1Zm-8 2a3 3 0 100-6 3 3 0 000 6Zm-7 0a1 1 0 010-2H4a1 1 0 110-2h1a1 1 0 110 2H4a1 1 0 010 2Zm1.64-7.95a1 1 0 011.41 0l.7.7a1 1 0 11-1.41 1.41l-.7-.7a1 1 0 010-1.41ZM12 18a1 1 0 011 1v1a1 1 0 11-2 0v-1a1 1 0 011-1Zm7.07-1.07a1 1 0 010 1.41l-.7.7a1 1 0 11-1.41-1.41l.7-.7a1 1 0 011.41 0ZM6.34 16.93a1 1 0 011.41 0l.7.7a1 1 0 11-1.41 1.41l-.7-.7a1 1 0 010-1.41Z"
                />
              </svg>
              <span>{{ t("settings.appearance.light") }}</span>
            </button>
            <button
              class="theme-btn"
              :class="{ active: theme === 'dark' }"
              @click="theme = 'dark'"
            >
              <svg class="theme-icon" viewBox="0 0 24 24" fill="currentColor">
                <path
                  d="M21 14.5A8.5 8.5 0 019.5 3a.9.9 0 00-.9.9 9.6 9.6 0 0010.5 10.5.9.9 0 00.9-.9Z"
                />
              </svg>
              <span>{{ t("settings.appearance.dark") }}</span>
            </button>
            <button
              class="theme-btn"
              :class="{ active: theme === 'system' }"
              @click="theme = 'system'"
            >
              <svg class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="2" y="3" width="20" height="14" rx="2" />
                <path d="M8 21h8M12 17v4" />
              </svg>
              <span>{{ t("settings.appearance.system") }}</span>
            </button>
          </div>
        </div>

        <!-- Language -->
        <div class="setting-row">
          <label class="setting-label">{{ t("settings.appearance.language") }}</label>
          <div class="language-options">
            <button
              class="lang-btn"
              :class="{ active: locale === 'zh-CN' }"
              @click="locale = 'zh-CN'"
            >
              中文
            </button>
            <button
              class="lang-btn"
              :class="{ active: locale === 'en-US' }"
              @click="locale = 'en-US'"
            >
              English
            </button>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-panel {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 20px;
  flex: 1;
  overflow: auto;
}

.settings-section {
  background: var(--color-panel-bg);
  border: 1px solid var(--color-panel-border);
  border-radius: 14px;
  padding: 20px;
  box-shadow: 0 4px 16px var(--color-panel-shadow);
}

.section-title {
  font-size: 15px;
  font-weight: 600;
  margin: 0 0 16px;
  color: var(--color-text);
}

/* About Section */
.about-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.app-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.app-name {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text);
}

.version-badge {
  font-size: 13px;
  font-weight: 500;
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--color-chip-bg);
  border: 1px solid var(--color-chip-border);
  color: var(--color-muted);
}

.about-actions {
  display: flex;
  gap: 10px;
}

.about-actions .ghost {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.btn-icon {
  width: 16px;
  height: 16px;
}

.update-available {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 16px;
  background: var(--color-success-bg);
  border: 1px solid var(--color-success-border);
  border-radius: 10px;
}

.update-message {
  font-size: 14px;
  color: var(--color-success-text);
  font-weight: 500;
}

.downloading {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 16px;
  background: var(--color-card-bg);
  border: 1px solid var(--color-card-border);
  border-radius: 10px;
}

.download-status {
  font-size: 14px;
  color: var(--color-muted);
}

.progress {
  width: 100%;
  height: 8px;
  background: var(--color-progress-bg);
  border-radius: 999px;
  overflow: hidden;
}

.progress-bar {
  height: 100%;
  background: var(--color-primary-bg);
  transition: width 0.3s ease;
}

.progress-text {
  font-size: 12px;
  color: var(--color-muted);
  text-align: right;
}

.download-complete {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 16px;
  background: var(--color-success-bg);
  border: 1px solid var(--color-success-border);
  border-radius: 10px;
}

.complete-message {
  font-size: 14px;
  color: var(--color-success-text);
  font-weight: 500;
}

.up-to-date {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: var(--color-card-bg);
  border: 1px solid var(--color-card-border);
  border-radius: 10px;
  color: var(--color-success-text);
  font-size: 14px;
}

.check-icon {
  width: 18px;
  height: 18px;
  color: var(--color-success-text);
}

/* Appearance Section */
.appearance-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.setting-row {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.setting-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-muted);
}

.theme-options {
  display: flex;
  gap: 10px;
}

.theme-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: 10px;
  border: 1px solid var(--color-input-border);
  background: var(--color-input-bg);
  color: var(--color-tab-text);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.theme-btn:hover {
  border-color: var(--color-input-focus);
}

.theme-btn.active {
  background: var(--color-tab-active-bg);
  color: var(--color-tab-active-text);
  border-color: transparent;
}

.theme-icon {
  width: 16px;
  height: 16px;
}

.language-options {
  display: flex;
  gap: 10px;
}

.lang-btn {
  padding: 10px 20px;
  border-radius: 10px;
  border: 1px solid var(--color-input-border);
  background: var(--color-input-bg);
  color: var(--color-tab-text);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.lang-btn:hover {
  border-color: var(--color-input-focus);
}

.lang-btn.active {
  background: var(--color-tab-active-bg);
  color: var(--color-tab-active-text);
  border-color: transparent;
}

/* Button styles */
.primary {
  border: none;
  border-radius: 10px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  background: var(--color-primary-bg);
  color: var(--color-primary-text);
  transition: all 0.2s ease;
}

.primary:hover:not(:disabled) {
  transform: translateY(-1px);
  filter: brightness(1.1);
}

.primary:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.ghost {
  background: transparent;
  border: 1px solid var(--color-ghost-border);
  color: var(--color-ghost-text);
  border-radius: 10px;
  padding: 8px 14px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.ghost:hover:not(:disabled) {
  transform: translateY(-1px);
  background: var(--color-card-bg);
}

.ghost:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

/* Responsive */
@media (max-width: 520px) {
  .theme-options {
    flex-wrap: wrap;
  }

  .version-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .update-available,
  .download-complete {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
}

.setting-hint {
  font-size: 13px;
  color: var(--color-muted);
  margin: 0;
}

.ides-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.ide-checkbox {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid var(--color-input-border);
  background: var(--color-input-bg);
  cursor: pointer;
  transition: all 0.2s ease;
  user-select: none;
  position: relative;
}

.ide-delete-btn {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
  padding: 3px;
  background: transparent;
  border: none;
  color: var(--color-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.ide-delete-btn:hover {
  background: rgba(239, 68, 68, 0.1);
  color: rgb(239, 68, 68);
}

.ide-checkbox:hover {
  border-color: var(--color-input-focus);
}

.ide-checkbox input[type="checkbox"] {
  width: 16px;
  height: 16px;
  margin: 0;
  cursor: pointer;
}

.ide-label {
  font-size: 14px;
  color: var(--color-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
