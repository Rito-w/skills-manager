<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const props = defineProps<{
  show: boolean;
  configs: Record<string, string>;
  enabled: Record<string, boolean>;
  statuses: Array<{ id: string; name: string; status: string; error?: string }>;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save", configs: Record<string, string>, enabled: Record<string, boolean>): void;
}>();

const localConfigs = ref<Record<string, string>>({});
const localEnabled = ref<Record<string, boolean>>({});

// Track API key visibility for each market
const apiKeyVisible = ref<Record<string, boolean>>({});

// Markets that require API key
const marketsNeedingKey: string[] = ["skillsmp"];

watch(
  () => props.show,
  (show) => {
    if (show) {
      localConfigs.value = { ...props.configs };
      localEnabled.value = { ...props.enabled };
      // Reset visibility state when modal opens
      apiKeyVisible.value = {};
    }
  }
);

function toggleApiKeyVisibility(marketId: string) {
  apiKeyVisible.value[marketId] = !apiKeyVisible.value[marketId];
}

function save() {
  emit("save", localConfigs.value, localEnabled.value);
  emit("close");
}

function getStatusLabel(status: string): string {
  if (status === "online") return t("marketSettings.online");
  if (status === "needs_key") return t("marketSettings.needsKey");
  return t("marketSettings.unavailable");
}
</script>

<template>
  <div v-if="show" class="modal-backdrop" @click.self="$emit('close')">
    <div class="modal">
      <div class="modal-title">{{ t("marketSettings.title") }}</div>
      
      <div class="market-list">
        <div v-for="market in statuses" :key="market.id" class="market-item">
          <div class="market-header">
            <label class="market-checkbox">
              <input 
                type="checkbox" 
                v-model="localEnabled[market.id]"
              />
              <span class="market-name">{{ market.name }}</span>
            </label>
            <span :class="['status-badge', market.status]">
              {{ getStatusLabel(market.status) }}
            </span>
          </div>
          
          <div v-if="market.error" class="market-error">
            {{ market.error }}
          </div>

          <!-- API Key input for markets that need it -->
          <div v-if="marketsNeedingKey.includes(market.id)" class="api-key-input">
            <label>{{ t("marketSettings.apiKey") }}</label>
            <div class="api-key-field">
              <input
                v-model="localConfigs[market.id]"
                :type="apiKeyVisible[market.id] ? 'text' : 'password'"
                class="input"
                :placeholder="t('marketSettings.apiKeyPlaceholder')"
              />
              <button
                type="button"
                class="toggle-visibility-btn"
                :title="apiKeyVisible[market.id] ? t('marketSettings.hideApiKey') : t('marketSettings.showApiKey')"
                @click="toggleApiKeyVisibility(market.id)"
              >
                <svg v-if="apiKeyVisible[market.id]" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/>
                  <line x1="1" y1="1" x2="23" y2="23"/>
                </svg>
                <svg v-else xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                  <circle cx="12" cy="12" r="3"/>
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="modal-actions">
        <button class="ghost" @click="$emit('close')">{{ t("marketSettings.cancel") }}</button>
        <button class="primary" @click="save">{{ t("marketSettings.save") }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.market-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-height: 400px;
  overflow-y: auto;
  margin: 16px 0;
}

.market-item {
  border: 1px solid var(--color-panel-border);
  border-radius: 8px;
  padding: 12px;
  background: var(--color-card-bg);
}

.market-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.market-checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.market-checkbox input {
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.market-name {
  font-weight: 600;
  font-size: 14px;
}

.status-badge {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 999px;
  font-weight: 500;
}

.status-badge.online {
  background: var(--color-success-bg);
  color: var(--color-success-text);
  border: 1px solid var(--color-success-border);
}

.status-badge.error,
.status-badge.unavailable {
  background: var(--color-error-bg);
  color: var(--color-error-text);
  border: 1px solid var(--color-error-border);
}

.status-badge.needs_key {
  background: var(--color-warning-bg, #fef3c7);
  color: var(--color-warning-text, #92400e);
  border: 1px solid var(--color-warning-border, #fcd34d);
}

.market-error {
  font-size: 12px;
  color: var(--color-error-text);
  margin-top: 8px;
  word-break: break-all;
}

.api-key-input {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 12px;
}

.api-key-input label {
  font-size: 12px;
  color: var(--color-muted);
}

.api-key-field {
  display: flex;
  align-items: center;
  gap: 8px;
}

.api-key-field .input {
  flex: 1;
}

.toggle-visibility-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--color-panel-border);
  border-radius: 6px;
  background: var(--color-card-bg);
  color: var(--color-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}

.toggle-visibility-btn:hover {
  background: var(--color-bg);
  color: var(--color-text);
  border-color: var(--color-text-muted);
}
</style>
