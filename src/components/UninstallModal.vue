<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{
  visible: boolean;
  targetName: string;
}>();

defineEmits<{
  (e: "confirm"): void;
  (e: "cancel"): void;
}>();

const { t } = useI18n();
</script>

<template>
  <div v-if="visible" class="modal-backdrop">
    <div class="modal">
      <div class="modal-title">{{ t("uninstallModal.title") }}</div>
      <div class="hint">{{ t("uninstallModal.hint") }}</div>
      <div class="card-link">{{ targetName }}</div>
      <div class="modal-actions">
        <button class="ghost" @click="$emit('cancel')">{{ t("uninstallModal.cancel") }}</button>
        <button class="primary" @click="$emit('confirm')">{{ t("uninstallModal.confirm") }}</button>
      </div>
    </div>
  </div>
</template>
