import type { IdeOption, LinkTarget, ProjectConfig } from "./types";

export function buildProjectLinkTargets(
  project: ProjectConfig,
  ideLabel: string,
  ideOptions: IdeOption[]
): LinkTarget[] {
  const detectedDir = project.detectedIdeDirs.find((item) => item.label === ideLabel);
  if (detectedDir) {
    const normalizedPath = detectedDir.absolutePath?.trim() || `${project.path}/${detectedDir.relativeDir}`;
    return [{ name: `${ideLabel} (${project.name})`, path: normalizedPath }];
  }

  const target = ideOptions.find((option) => option.label === ideLabel);
  if (!target) return [];

  const dir = target.globalDir.trim();
  if (!dir || dir.startsWith("/")) {
    return [];
  }

  return [{ name: `${target.label} (${project.name})`, path: `${project.path}/${dir}` }];
}
