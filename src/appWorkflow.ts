export interface RecentProject {
  path: string;
  name: string;
  last_opened: string;
}

export interface OpenFileLike {
  path: string;
  name: string;
}

export const MAX_RECENT_PROJECTS = 8;
export const MAX_LOG_LINES = 1000;
export const MAX_LOG_CHARACTERS = 2_000_000;

export function trimLog(lines: string[]) {
  const trimmed = lines.slice(-MAX_LOG_LINES);
  let characters = trimmed.reduce((total, line) => total + line.length, 0);
  while (characters > MAX_LOG_CHARACTERS && trimmed.length > 1) {
    characters -= trimmed.shift()?.length ?? 0;
  }
  return trimmed;
}

export function appendLog(lines: string[], line: string) {
  return trimLog([...lines, line]);
}

export function normalizeRecentProjects(projects: RecentProject[]) {
  const byPath = new Map<string, RecentProject>();

  for (const project of projects) {
    const path = project.path.trim();
    if (!path) continue;

    const existing = byPath.get(path);
    const next = {
      path,
      name: project.name || path.split(/[\\/]/).filter(Boolean).pop() || path,
      last_opened: project.last_opened || String(Date.now()),
    };

    if (!existing || Number(next.last_opened) >= Number(existing.last_opened)) {
      byPath.set(path, next);
    }
  }

  return [...byPath.values()]
    .sort((a, b) => Number(b.last_opened) - Number(a.last_opened))
    .slice(0, MAX_RECENT_PROJECTS);
}

export function getNextFileAfterClose<T extends OpenFileLike>(
  files: T[],
  activePath: string | null,
  closedPath: string,
): string | null {
  if (activePath !== closedPath) {
    return activePath;
  }

  const remaining = files.filter((file) => file.path !== closedPath);
  const index = files.findIndex((file) => file.path === closedPath);
  const fallback =
    remaining[index] ??
    remaining[index - 1] ??
    remaining[remaining.length - 1] ??
    null;

  return fallback ? fallback.path : null;
}
