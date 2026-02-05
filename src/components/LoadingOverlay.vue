<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{
  visible: boolean;
  text: string;
}>();

const { t } = useI18n();
</script>

<template>
  <div v-if="visible" class="modal-backdrop">
    <div class="modal">
      <div class="modal-title">{{ t("loading.title") }}</div>
      <div class="progress">
        <span class="progress-bar" />
      </div>
      <div class="hint">{{ text }}</div>
    </div>
  </div>
</template>
