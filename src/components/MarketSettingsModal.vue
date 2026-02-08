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

// Markets that require API key
const marketsNeedingKey: string[] = ["skillsmp"];

watch(
  () => props.show,
  (show) => {
    if (show) {
      localConfigs.value = { ...props.configs };
      localEnabled.value = { ...props.enabled };
    }
  }
);

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
            <input 
              v-model="localConfigs[market.id]" 
              type="password" 
              class="input" 
              :placeholder="t('marketSettings.apiKeyPlaceholder')"
            />
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
</style>
