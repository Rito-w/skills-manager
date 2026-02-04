<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { homeDir, join } from "@tauri-apps/api/path";

type RemoteSkill = {
  id: string;
  name: string;
  namespace: string;
  sourceUrl: string;
  description: string;
  author: string;
  installs: number;
  stars: number;
};

type InstallResult = {
  installedPath: string;
  linked: string[];
  skipped: string[];
};

type LocalSkill = {
  id: string;
  name: string;
  description: string;
  path: string;
  source: string;
  ide?: string;
  usedBy: string[];
};

type IdeSkill = {
  id: string;
  name: string;
  path: string;
  ide: string;
  source: string;
};

type Overview = {
  managerSkills: LocalSkill[];
  ideSkills: IdeSkill[];
};

type IdeOption = {
  id: string;
  label: string;
  globalDir: string;
};

type LinkTarget = {
  name: string;
  path: string;
};

const defaultIdeOptions: IdeOption[] = [
  { id: "antigravity", label: "Antigravity", globalDir: ".agent/skills" },
  { id: "claude", label: "Claude", globalDir: ".claude/skills" },
  { id: "codebuddy", label: "CodeBuddy", globalDir: ".codebuddy/skills" },
  { id: "codex", label: "Codex", globalDir: ".codex/skills" },
  { id: "cursor", label: "Cursor", globalDir: ".cursor/skills" },
  { id: "qoder", label: "Qoder", globalDir: ".qoder/skills" },
  { id: "trae", label: "Trae", globalDir: ".trae/skills" },
  { id: "vscode", label: "VSCode", globalDir: ".github/skills" },
  { id: "windsurf", label: "Windsurf", globalDir: ".windsurf/skills" }
];

const activeTab = ref<"local" | "market" | "ide">("local");

const query = ref("");
const results = ref<RemoteSkill[]>([]);
const total = ref(0);
const limit = ref(20);
const offset = ref(0);
const loading = ref(false);
const error = ref<string | null>(null);
const installingId = ref<string | null>(null);
const installMessage = ref<string | null>(null);

const localSkills = ref<LocalSkill[]>([]);
const ideSkills = ref<IdeSkill[]>([]);
const selectedIdeFilter = ref("Antigravity");
const ideOptions = ref<IdeOption[]>([]);
const customIdeName = ref("");
const customIdeDir = ref("");
const showInstallModal = ref(false);
const installTargetSkill = ref<LocalSkill | null>(null);
const installTargetIde = ref<string[]>([]);
const showUninstallModal = ref(false);
const uninstallTargetPath = ref("");
const uninstallTargetName = ref("");
const updatingId = ref<string | null>(null);
const localLoading = ref(false);
const localError = ref<string | null>(null);

const hasMore = computed(() => results.value.length < total.value);
const localSkillNameSet = computed(() => {
  const set = new Set<string>();
  for (const skill of localSkills.value) {
    const key = skill.name.trim().toLowerCase();
    if (key) set.add(key);
  }
  return set;
});

const ideKey = "skillsManager.ideOptions";

function loadIdeOptions(): IdeOption[] {
  try {
    const raw = localStorage.getItem(ideKey);
    if (!raw) return [...defaultIdeOptions];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [...defaultIdeOptions];
    const custom = parsed.filter(
      (item) =>
        item &&
        typeof item.label === "string" &&
        typeof item.globalDir === "string"
    );
    return [...defaultIdeOptions, ...custom].sort((a, b) =>
      a.label.localeCompare(b.label)
    );
  } catch {
    return [...defaultIdeOptions];
  }
}

function saveIdeOptions(custom: IdeOption[]) {
  localStorage.setItem(ideKey, JSON.stringify(custom));
}

function refreshIdeOptions() {
  ideOptions.value = loadIdeOptions();
  if (!ideOptions.value.find((item) => item.label === selectedIdeFilter.value)) {
    selectedIdeFilter.value = ideOptions.value[0]?.label ?? "Antigravity";
  }
}

function addCustomIde() {
  const name = customIdeName.value.trim();
  const dir = customIdeDir.value.trim();
  if (!name || !dir) {
    localError.value = "请填写 IDE 名称和目录";
    return;
  }
  const existingCustom = ideOptions.value
    .filter((item) => !defaultIdeOptions.find((def) => def.id === item.id))
    .filter((item) => item.label !== name);
  const id = `custom-${name.toLowerCase().replace(/\\s+/g, "-")}`;
  const nextCustom = [...existingCustom, { id, label: name, globalDir: dir }].sort((a, b) =>
    a.label.localeCompare(b.label)
  );
  saveIdeOptions(nextCustom);
  customIdeName.value = "";
  customIdeDir.value = "";
  refreshIdeOptions();
  void scanLocalSkills();
}

function removeCustomIde(label: string) {
  const customOnly = ideOptions.value.filter(
    (item) => !defaultIdeOptions.find((def) => def.id === item.id)
  );
  const nextCustom = customOnly.filter((item) => item.label !== label);
  saveIdeOptions(nextCustom);
  refreshIdeOptions();
  void scanLocalSkills();
}

async function buildInstallBaseDir(): Promise<string> {
  const home = await homeDir();
  return join(home, ".skills-manager/skills");
}

async function buildLinkTargets(targetLabel: string): Promise<LinkTarget[]> {
  const home = await homeDir();
  const target = ideOptions.value.find((option) => option.label === targetLabel);
  if (!target) return [];
  return [
    {
      name: target.label,
      path: await join(home, target.globalDir)
    }
  ];
}

async function searchMarketplace(reset = true) {
  if (loading.value) return;
  loading.value = true;
  error.value = null;
  installMessage.value = null;

  const nextOffset = reset ? 0 : offset.value + limit.value;

  try {
    const response = await invoke("search_marketplace", {
      query: query.value,
      limit: limit.value,
      offset: nextOffset
    });
    const data = response as { skills: RemoteSkill[]; total: number; limit: number; offset: number };

    if (reset) {
      results.value = data.skills;
    } else {
      results.value = [...results.value, ...data.skills];
    }

    total.value = data.total;
    offset.value = data.offset;
  } catch (err) {
    error.value = err instanceof Error ? err.message : "搜索失败";
  } finally {
    loading.value = false;
  }
}

async function downloadSkill(skill: RemoteSkill) {
  if (installingId.value) return;

  installingId.value = skill.id;
  error.value = null;
  installMessage.value = null;

  try {
    const installBaseDir = await buildInstallBaseDir();
    const result = (await invoke("download_marketplace_skill", {
      request: {
        sourceUrl: skill.sourceUrl,
        skillName: skill.name,
        installBaseDir
      }
    })) as { installedPath: string };

    installMessage.value = `已下载到 ${result.installedPath}`;
    await scanLocalSkills();
  } catch (err) {
    error.value = err instanceof Error ? err.message : "下载失败";
  } finally {
    installingId.value = null;
  }
}

async function updateSkill(skill: RemoteSkill) {
  if (updatingId.value) return;

  updatingId.value = skill.id;
  error.value = null;
  installMessage.value = null;

  try {
    const installBaseDir = await buildInstallBaseDir();
    const result = (await invoke("update_marketplace_skill", {
      request: {
        sourceUrl: skill.sourceUrl,
        skillName: skill.name,
        installBaseDir
      }
    })) as { installedPath: string };

    installMessage.value = `已更新 ${result.installedPath}`;
    await scanLocalSkills();
  } catch (err) {
    error.value = err instanceof Error ? err.message : "更新失败";
  } finally {
    updatingId.value = null;
  }
}

async function scanLocalSkills() {
  if (localLoading.value) return;
  localLoading.value = true;
  localError.value = null;

  try {
    const response = (await invoke("scan_overview", {
      request: {
        projectDir: null,
        ideDirs: ideOptions.value.map((item) => ({
          label: item.label,
          relativeDir: item.globalDir
        }))
      }
    })) as Overview;
    localSkills.value = response.managerSkills;
    ideSkills.value = response.ideSkills;
  } catch (err) {
    localError.value = err instanceof Error ? err.message : "扫描失败";
  } finally {
    localLoading.value = false;
  }
}

async function linkSkill(skill: LocalSkill, ideLabel: string) {
  if (installingId.value) return;

  installingId.value = skill.id;
  localError.value = null;
  installMessage.value = null;

  try {
    const linkTargets = await buildLinkTargets(ideLabel);
    if (linkTargets.length === 0) {
      localError.value = "请选择有效的 IDE";
      return;
    }
    const result = (await invoke("link_local_skill", {
      request: {
        skillPath: skill.path,
        skillName: skill.name,
        linkTargets
      }
    })) as InstallResult;

    const linkedCount = result.linked.length;
    const skippedCount = result.skipped.length;
    installMessage.value = `已链接 ${linkedCount}，跳过 ${skippedCount}`;
    await scanLocalSkills();
  } catch (err) {
    localError.value = err instanceof Error ? err.message : "链接失败";
  } finally {
    installingId.value = null;
  }
}

function openInstallModal(skill: LocalSkill) {
  installTargetSkill.value = skill;
  installTargetIde.value = ideOptions.value.slice(0, 2).map((item) => item.label);
  showInstallModal.value = true;
}

async function confirmInstallToIde() {
  if (!installTargetSkill.value || installTargetIde.value.length === 0) {
    localError.value = "请选择至少一个 IDE";
    return;
  }
  for (const label of installTargetIde.value) {
    await linkSkill(installTargetSkill.value, label);
  }
  showInstallModal.value = false;
  installTargetSkill.value = null;
}

function closeInstallModal() {
  showInstallModal.value = false;
  installTargetSkill.value = null;
}

const filteredIdeSkills = computed(() =>
  ideSkills.value.filter((skill) => skill.ide === selectedIdeFilter.value)
);

async function uninstallSkill(targetPath: string) {
  uninstallTargetPath.value = targetPath;
  uninstallTargetName.value = targetPath.split("/").pop() || targetPath;
  showUninstallModal.value = true;
}

async function confirmUninstall() {
  try {
    const message = (await invoke("uninstall_skill", {
      request: {
        targetPath: uninstallTargetPath.value,
        projectDir: null,
        ideDirs: ideOptions.value.map((item) => ({
          label: item.label,
          relativeDir: item.globalDir
        }))
      }
    })) as string;
    installMessage.value = message;
    await scanLocalSkills();
  } catch (err) {
    localError.value = err instanceof Error ? err.message : "卸载失败";
  } finally {
    showUninstallModal.value = false;
    uninstallTargetPath.value = "";
    uninstallTargetName.value = "";
  }
}

function cancelUninstall() {
  showUninstallModal.value = false;
  uninstallTargetPath.value = "";
  uninstallTargetName.value = "";
}

onMounted(() => {
  refreshIdeOptions();
  void searchMarketplace(true);
  void scanLocalSkills();
});
</script>

<template>
  <div class="app">
    <header class="header">
      <div>
        <div class="title">Skills Manager</div>
        <div class="subtitle">面向多 IDE 的 Skills 管理器 · Marketplace + 本地链接</div>
      </div>
      <div class="tabs">
        <button
          class="tab"
          :class="{ active: activeTab === 'local' }"
          @click="activeTab = 'local'"
        >
          已有 Skills
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'market' }"
          @click="activeTab = 'market'"
        >
          Market
        </button>
        <button
          class="tab"
          :class="{ active: activeTab === 'ide' }"
          @click="activeTab = 'ide'"
        >
          IDE 浏览
        </button>
      </div>
    </header>

    <template v-if="activeTab === 'local'">
      <section class="panel">
        <div class="panel-title">已有 Skills</div>
        <div class="hint">这里展示 Skills Manager 本地仓库中的 skills。</div>
        <div class="actions">
          <div class="hint">下载到本地仓库后会自动出现。</div>
        </div>
        <div v-if="installMessage" class="message success">{{ installMessage }}</div>
        <div v-if="localError" class="message error">{{ localError }}</div>
        <div v-if="localLoading" class="hint">扫描中...</div>
        <div v-if="!localLoading && localSkills.length === 0" class="hint">暂未扫描到本地 skills</div>
        <div v-if="localSkills.length > 0" class="cards">
          <article v-for="skill in localSkills" :key="skill.id" class="card">
            <div class="card-header">
              <div>
                <div class="card-title">{{ skill.name }}</div>
                <div class="card-meta">
                  Skills Manager ·
                  {{ skill.usedBy.length > 0 ? `已关联 ${skill.usedBy.join(", ")}` : "未关联" }}
                </div>
              </div>
              <button class="primary" :disabled="installingId === skill.id" @click="openInstallModal(skill)">
                {{ installingId === skill.id ? "处理中" : "安装" }}
              </button>
            </div>
            <p class="card-desc">{{ skill.description }}</p>
            <div class="card-link">{{ skill.path }}</div>
          </article>
        </div>
      </section>
    </template>

    <template v-else-if="activeTab === 'market'">
      <section class="panel">
        <div class="panel-title">Marketplace 搜索</div>
        <div class="search-row">
          <input
            v-model="query"
            class="input"
            placeholder="搜索 skills（支持名称 / 描述 / 作者）"
            @keydown.enter.prevent="searchMarketplace(true)"
            :disabled="loading"
          />
          <button class="primary" @click="searchMarketplace(true)" :disabled="loading">
            {{ loading ? "搜索中..." : "搜索" }}
          </button>
          <button class="ghost" @click="searchMarketplace(true)" :disabled="loading">
            {{ loading ? "加载中" : "刷新" }}
          </button>
        </div>
        <div v-if="error" class="message error">{{ error }}</div>
        <div v-if="installMessage" class="message success">{{ installMessage }}</div>
      </section>

      <section class="panel">
        <div class="panel-title">结果列表</div>
        <div v-if="loading && results.length === 0" class="hint">加载中...</div>
        <div v-if="results.length === 0 && !loading" class="hint">暂无结果</div>

        <div class="cards">
          <article v-for="skill in results" :key="skill.id" class="card">
            <div class="card-header">
              <div>
                <div class="card-title">{{ skill.name }}</div>
                <div class="card-meta">{{ skill.author }} · ★ {{ skill.stars }} · {{ skill.installs }} installs</div>
              </div>
              <template v-if="localSkillNameSet.has(skill.name.trim().toLowerCase())">
                <button
                  class="ghost"
                  :disabled="updatingId === skill.id"
                  @click="updateSkill(skill)"
                >
                  {{ updatingId === skill.id ? "更新中..." : "更新" }}
                </button>
              </template>
              <template v-else>
                <button
                  class="primary"
                  :disabled="installingId === skill.id"
                  @click="downloadSkill(skill)"
                >
                  {{ installingId === skill.id ? "下载中" : "下载到本地" }}
                </button>
              </template>
            </div>
            <p class="card-desc">{{ skill.description }}</p>
            <div class="card-link">{{ skill.sourceUrl }}</div>
          </article>
        </div>

        <div v-if="hasMore" class="more">
          <button class="ghost" @click="searchMarketplace(false)" :disabled="loading">
            加载更多
          </button>
        </div>
      </section>
    </template>

    <template v-else>
      <section class="panel">
        <div class="panel-title">IDE 浏览</div>
        <div class="row">
          <div class="select">
            <select v-model="selectedIdeFilter">
              <option v-for="option in ideOptions" :key="option.id" :value="option.label">
                {{ option.label }}
              </option>
            </select>
          </div>
          <div class="hint">切换 IDE 查看其技能列表。</div>
        </div>
        <div class="hint">添加自定义 IDE（名称 + 相对用户目录的 skills 路径）。</div>
        <div class="row">
          <input v-model="customIdeName" class="input small" placeholder="IDE 名称" />
          <input v-model="customIdeDir" class="input small" placeholder="例如 .myide/skills" />
          <button class="primary" @click="addCustomIde">添加 IDE</button>
        </div>
        <div v-if="ideOptions.some((option) => option.id.startsWith('custom-'))" class="chips">
          <div v-for="option in ideOptions" :key="option.id" class="chip" v-if="option.id.startsWith('custom-')">
            <span>{{ option.label }}</span>
            <button class="ghost" @click="removeCustomIde(option.label)">删除</button>
          </div>
        </div>
        <div v-if="localError" class="message error">{{ localError }}</div>
        <div v-if="localLoading" class="hint">加载中...</div>
        <div v-if="!localLoading && filteredIdeSkills.length === 0" class="hint">该 IDE 暂无 skills</div>
        <div v-if="filteredIdeSkills.length > 0" class="cards">
          <article v-for="skill in filteredIdeSkills" :key="skill.id" class="card">
            <div class="card-header">
              <div>
                <div class="card-title">{{ skill.name }}</div>
                <div class="card-meta">{{ skill.ide }} · {{ skill.source === "link" ? "链接" : "本地" }}</div>
              </div>
              <button class="ghost" @click="uninstallSkill(skill.path)">卸载</button>
            </div>
            <div class="card-link">{{ skill.path }}</div>
          </article>
        </div>
      </section>
    </template>

    <div v-if="showInstallModal" class="modal-backdrop">
      <div class="modal">
        <div class="modal-title">选择安装目标 IDE</div>
        <div class="grid">
          <label v-for="option in ideOptions" :key="option.id" class="checkbox">
            <input v-model="installTargetIde" type="checkbox" :value="option.label" />
            {{ option.label }}
          </label>
        </div>
        <div class="modal-actions">
          <button class="ghost" @click="closeInstallModal">取消</button>
          <button class="primary" @click="confirmInstallToIde">确认安装</button>
        </div>
      </div>
    </div>

    <div v-if="showUninstallModal" class="modal-backdrop">
      <div class="modal">
        <div class="modal-title">确认卸载</div>
        <div class="hint">将移除该 IDE 下的技能目录或软链接，无法恢复。</div>
        <div class="card-link">{{ uninstallTargetName }}</div>
        <div class="modal-actions">
          <button class="ghost" @click="cancelUninstall">取消</button>
          <button class="primary" @click="confirmUninstall">确认卸载</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
:root {
  font-family: "IBM Plex Sans", "Source Sans 3", "Noto Sans", sans-serif;
  font-size: 16px;
  line-height: 1.5;
  font-weight: 400;
  color: #1a1b1f;
  background-color: #f5f5f0;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

body {
  margin: 0;
  background: #f5f5f0;
}

#app {
  min-height: 100vh;
}
</style>

<style scoped>
.app {
  padding: 32px 40px 48px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
}

.title {
  font-size: 24px;
  font-weight: 600;
  letter-spacing: 0.2px;
}

.subtitle {
  color: #5e616e;
  font-size: 14px;
  margin-top: 6px;
}

.panel {
  background: #ffffff;
  border: 1px solid #e6e6e0;
  border-radius: 14px;
  padding: 20px;
  box-shadow: 0 4px 16px rgba(16, 18, 27, 0.04);
}

.tabs {
  display: inline-flex;
  gap: 6px;
  padding: 6px;
  border-radius: 999px;
  border: 1px solid #e2e2dc;
  background: #f8f8f3;
}

.tab {
  border: none;
  background: transparent;
  padding: 8px 16px;
  border-radius: 999px;
  font-size: 13px;
  color: #4e515c;
  cursor: pointer;
}

.tab.active {
  background: #1e1f24;
  color: #ffffff;
}

.panel-title {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 12px;
}

.search-row {
  display: flex;
  gap: 12px;
  align-items: center;
}

.input {
  flex: 1;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid #dcdcd6;
  background: #fcfcfa;
  font-size: 14px;
}

.input.small {
  max-width: 220px;
}

.input:focus {
  outline: none;
  border-color: #6a6f7a;
}

.row {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-top: 12px;
}

.chips {
  display: grid;
  gap: 8px;
  margin: 12px 0;
}

.chip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border: 1px solid #e2e2dc;
  border-radius: 10px;
  padding: 8px 12px;
  background: #fdfdf9;
  font-size: 13px;
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(17, 18, 22, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.modal {
  background: #ffffff;
  border-radius: 14px;
  border: 1px solid #e5e5df;
  padding: 20px;
  width: min(420px, 100%);
  box-shadow: 0 10px 30px rgba(16, 18, 27, 0.15);
}

.modal-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 12px;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 16px;
}

.actions {
  margin-top: 10px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 10px 16px;
}

.radio,
.checkbox {
  display: flex;
  gap: 10px;
  align-items: center;
  font-size: 14px;
}

.select select {
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid #dcdcd6;
  background: #fcfcfa;
  min-width: 380px;
  font-size: 15px;
}

.message {
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  font-size: 13px;
}

.message.error {
  background: #fff4f2;
  color: #b73c2f;
  border: 1px solid #f2c9c2;
}

.message.success {
  background: #f1f7f1;
  color: #2e6b3d;
  border: 1px solid #c8e0cb;
}

.hint {
  font-size: 13px;
  color: #6b6f7a;
  margin-top: 6px;
}

.cards {
  display: grid;
  gap: 14px;
  margin-top: 12px;
}

.card {
  border: 1px solid #ecece6;
  border-radius: 12px;
  padding: 14px 16px;
  background: #fafaf7;
}

.card-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
}

.card-title {
  font-size: 16px;
  font-weight: 600;
}

.card-meta {
  font-size: 12px;
  color: #7a7f8a;
  margin-top: 4px;
}

.card-desc {
  margin: 10px 0 0;
  font-size: 14px;
  color: #2d2f36;
}

.card-link {
  margin-top: 8px;
  font-size: 12px;
  color: #8a8f9a;
  word-break: break-all;
}

.primary,
.ghost {
  border: none;
  border-radius: 10px;
  padding: 8px 14px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s ease, color 0.2s ease, border 0.2s ease;
}

.primary {
  background: #1e1f24;
  color: #ffffff;
}

.primary:disabled,
.ghost:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.ghost {
  background: transparent;
  border: 1px solid #dcdcd6;
  color: #2b2f38;
}

.more {
  display: flex;
  justify-content: center;
  margin-top: 16px;
}

@media (max-width: 720px) {
  .app {
    padding: 24px;
  }

  .header {
    flex-direction: column;
    align-items: flex-start;
  }

  .search-row {
    flex-direction: column;
    align-items: stretch;
  }

  .select select {
    min-width: 0;
    width: 100%;
  }
}
</style>
