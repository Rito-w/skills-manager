import type { IdeOption, MarketStatus } from "./types";

/**
 * Default IDE options available for skill installation
 */
export const defaultIdeOptions: IdeOption[] = [
  { id: "antigravity", label: "Antigravity", globalDir: ".gemini/antigravity/skills" },
  { id: "claude", label: "Claude", globalDir: ".claude/skills" },
  { id: "codebuddy", label: "CodeBuddy", globalDir: ".codebuddy/skills" },
  { id: "codex", label: "Codex", globalDir: ".codex/skills" },
  { id: "cursor", label: "Cursor", globalDir: ".cursor/skills" },
  { id: "kiro", label: "Kiro", globalDir: ".kiro/skills" },
  { id: "qoder", label: "Qoder", globalDir: ".qoder/skills" },
  { id: "trae", label: "Trae", globalDir: ".trae/skills" },
  { id: "vscode", label: "VSCode", globalDir: ".github/skills" },
  { id: "windsurf", label: "Windsurf", globalDir: ".windsurf/skills" }
];

/**
 * LocalStorage keys
 */
export const STORAGE_KEYS = {
  ideOptions: "skillsManager.ideOptions",
  installTargets: "skillsManager.lastInstallTargets",
  marketConfigs: "skillsManager.marketConfigs",
  enabledMarkets: "market-enabled"
} as const;

/**
 * Cache time-to-live in milliseconds (10 minutes)
 */
export const CACHE_TTL_MS = 10 * 60 * 1000;

/**
 * Default market statuses
 */
export const defaultMarketStatuses: MarketStatus[] = [
  { id: "claude-plugins", name: "Claude Plugins", status: "online" },
  { id: "skillsllm", name: "SkillsLLM", status: "online" },
  { id: "skillsmp", name: "SkillsMP", status: "needs_key" }
];

/**
 * Default enabled markets
 */
export const defaultEnabledMarkets: Record<string, boolean> = {
  "claude-plugins": true,
  "skillsllm": true,
  "skillsmp": false // Disabled by default until API key is provided
};
