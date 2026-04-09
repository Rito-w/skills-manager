<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { LocalSkill } from "../composables/types";

defineProps<{
  visible: boolean;
  skill: LocalSkill | null;
}>();

defineEmits<{
  (e: "adopt"): void;
  (e: "search"): void;
  (e: "cancel"): void;
}>();

const { t } = useI18n();
</script>

<template>
  <div v-if="visible && skill" class="modal-backdrop">
    <div class="modal">
      <div class="modal-title">
        {{ t("unmanagedModal.title") }}
      </div>
      <div class="hint">
        {{ t("unmanagedModal.hint") }}
      </div>
      <div class="card-link">{{ skill.name }}</div>
      <div class="modal-actions" style="margin-top: 16px;">
        <button class="ghost" @click="$emit('cancel')">{{ t("unmanagedModal.cancel") }}</button>
        <div style="flex: 1;"></div>
        <button class="primary" style="background: var(--color-primary-bg); color: var(--color-primary-text);" @click="$emit('search')">
          {{ t("unmanagedModal.searchMarket") }}
        </button>
        <button class="primary" @click="$emit('adopt')">
          {{ t("unmanagedModal.adopt") }}
        </button>
      </div>
    </div>
  </div>
</template>
