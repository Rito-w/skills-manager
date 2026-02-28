/**
 * Remote skill from marketplace
 */
export type RemoteSkill = {
  id: string;
  name: string;
  namespace: string;
  sourceUrl: string;
  description: string;
  author: string;
  installs: number;
  stars: number;
  marketId: string;
  marketLabel: string;
};

/**
 * Market connection status
 */
export type MarketStatus = {
  id: string;
  name: string;
  status: "online" | "error" | "needs_key";
  error?: string;
};

/**
 * Result of skill installation
 */
export type InstallResult = {
  installedPath: string;
  linked: string[];
  skipped: string[];
};

/**
 * Local skill managed by skills-manager
 */
export type LocalSkill = {
  id: string;
  name: string;
  description: string;
  path: string;
  source: string;
  ide?: string;
  usedBy: string[];
};

/**
 * Skill in IDE directory
 */
export type IdeSkill = {
  id: string;
  name: string;
  path: string;
  ide: string;
  source: string;
};

/**
 * Overview of all skills
 */
export type Overview = {
  managerSkills: LocalSkill[];
  ideSkills: IdeSkill[];
};

/**
 * IDE configuration option
 */
export type IdeOption = {
  id: string;
  label: string;
  globalDir: string;
};

/**
 * Link target for skill installation
 */
export type LinkTarget = {
  name: string;
  path: string;
};

/**
 * Download task in queue
 */
export type DownloadTask = {
  id: string;
  name: string;
  sourceUrl: string;
  status: "pending" | "downloading" | "done" | "error";
  error?: string;
};
