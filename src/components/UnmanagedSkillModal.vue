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
        管理未纳管技能
      </div>
      <div class="hint">
        该技能存在于项目本地但尚未受本应用统一管理，您希望执行什么操作？
      </div>
      <div class="card-link">{{ skill.name }}</div>
      <div class="modal-actions" style="margin-top: 16px;">
        <button class="ghost" @click="$emit('cancel')">{{ t("uninstallModal.cancel") || "取消" }}</button>
        <div style="flex: 1;"></div>
        <button class="primary" style="background: var(--color-primary-bg); color: var(--color-primary-text);" @click="$emit('search')">
          去市场检索
        </button>
        <button class="primary" @click="$emit('adopt')">
          纳入统一管理
        </button>
      </div>
    </div>
  </div>
</template>
