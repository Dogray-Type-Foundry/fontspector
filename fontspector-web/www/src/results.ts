// Pure data helpers for the pane-based report view: aggregation of per-file
// results into per-check summaries, filtering, grouping and message collation.
import { reactive } from "vue";
import { STATUS_ORDER, worseThan } from "./constants";
import { CheckResult, Status, StatusCode } from "./types";
import { state } from "./store";

export const FAMILY = "Family checks";
export const ISSUE_LEVELS: StatusCode[] = ["FATAL", "ERROR", "FAIL", "WARN"];
export type GroupBy = "severity" | "section" | "file" | "none";

export const filters = reactive({
  search: "",
  severities: new Set<StatusCode>(["FATAL", "ERROR", "FAIL", "WARN", "INFO"]),
  files: new Set<string>(), // empty = all files
  sections: new Set<string>(), // empty = all sections
  fixableOnly: false,
  groupBy: "severity" as GroupBy,
});

/** Selected check id in the report view; null shows the overview. */
export const selection = reactive({ checkId: null as string | null });

export function resetFilters() {
  filters.search = "";
  filters.severities = new Set(["FATAL", "ERROR", "FAIL", "WARN", "INFO"]);
  filters.files = new Set();
  filters.sections = new Set();
  filters.fixableOnly = false;
}

/** Family-wide results carry a non-font "filename" (e.g. "A collection"). */
export function fileOf(r: CheckResult): string {
  return r.filename && r.filename in state.fonts ? r.filename : FAMILY;
}

export function fontCount(): number {
  return Object.keys(state.fonts).length;
}

export function baseName(path: string): string {
  return path.split("/").slice(-1)[0];
}

export function sectionOf(r: CheckResult): string {
  return r.section || "Other";
}

export function severityRank(s: StatusCode): number {
  return STATUS_ORDER.indexOf(s);
}

/** A result can be hotfixed: the check has a fixer, it is WARN or worse, and it belongs to a real file. */
export function isFixable(r: CheckResult): boolean {
  return (
    r.hotfix_available && worseThan(r.worst_status, "WARN") && fileOf(r) !== FAMILY
  );
}

/** One check, aggregated over the files it ran on. */
export interface CheckSummary {
  id: string;
  name: string;
  rationale: string;
  section: string;
  worst: StatusCode;
  results: CheckResult[];
  fixable: boolean;
}

function summarize(results: CheckResult[]): CheckSummary {
  const first = results[0];
  let worst: StatusCode = "SKIP";
  for (const r of results) if (worseThan(r.worst_status, worst)) worst = r.worst_status;
  return {
    id: first.check_id,
    name: first.check_name,
    rationale: first.check_rationale,
    section: sectionOf(first),
    worst,
    results,
    fixable: results.some(isFixable),
  };
}

export function aggregate(results: CheckResult[]): CheckSummary[] {
  const byId = new Map<string, CheckResult[]>();
  for (const r of results) {
    if (!byId.has(r.check_id)) byId.set(r.check_id, []);
    byId.get(r.check_id)!.push(r);
  }
  return [...byId.values()].map(summarize).sort(compareChecks);
}

function compareChecks(a: CheckSummary, b: CheckSummary): number {
  return severityRank(a.worst) - severityRank(b.worst) || a.name.localeCompare(b.name);
}

function matchesSearch(c: CheckSummary, q: string): boolean {
  if (!q) return true;
  q = q.toLowerCase();
  if (c.name.toLowerCase().includes(q) || c.id.toLowerCase().includes(q)) return true;
  return c.results.some((r) =>
    r.subresults.some(
      (s) =>
        (s.message || "").toLowerCase().includes(q) ||
        (s.code || "").toLowerCase().includes(q),
    ),
  );
}

/**
 * Apply every filter except the severity filter (so severity chips can show
 * counts for what the other filters leave visible).
 */
export function applyNonSeverityFilters(results: CheckResult[]): CheckSummary[] {
  const inFiles =
    filters.files.size === 0 ? results : results.filter((r) => filters.files.has(fileOf(r)));
  return aggregate(inFiles).filter(
    (c) =>
      (filters.sections.size === 0 || filters.sections.has(c.section)) &&
      (!filters.fixableOnly || c.fixable) &&
      matchesSearch(c, filters.search.trim()),
  );
}

export function countBySeverity(checks: CheckSummary[]): Record<StatusCode, number> {
  const counts = Object.fromEntries(STATUS_ORDER.map((s) => [s, 0])) as Record<StatusCode, number>;
  for (const c of checks) counts[c.worst]++;
  return counts;
}

export interface CheckGroup {
  key: string;
  label: string;
  severity?: StatusCode;
  checks: CheckSummary[];
}

export function groupChecks(checks: CheckSummary[], by: GroupBy): CheckGroup[] {
  if (by === "none") return [{ key: "all", label: "All checks", checks }];
  if (by === "severity") {
    return STATUS_ORDER.map((s) => ({
      key: s,
      label: s,
      severity: s,
      checks: checks.filter((c) => c.worst === s),
    })).filter((g) => g.checks.length > 0);
  }
  const groups = new Map<string, CheckSummary[]>();
  const add = (key: string, c: CheckSummary) => {
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(c);
  };
  for (const c of checks) {
    if (by === "section") add(c.section, c);
    else for (const r of c.results) add(fileOf(r), summarize([r]));
  }
  return [...groups.entries()]
    .sort(([a], [b]) => (a === FAMILY ? -1 : b === FAMILY ? 1 : a.localeCompare(b)))
    .map(([key, cs]) => ({
      key,
      label: by === "file" ? baseName(key) : key,
      // Re-filter by severity: a per-file slice may be milder than the check overall.
      checks: cs.filter((c) => filters.severities.has(c.worst)).sort(compareChecks),
    }))
    .filter((g) => g.checks.length > 0);
}

/** A distinct message, with every file that produced it. */
export interface Finding {
  key: string;
  severity: StatusCode;
  code: string | null;
  message: string;
  status: Status;
  files: string[];
}

export function collateFindings(
  results: CheckResult[],
  signatureExtra: (s: Status) => string = () => "",
): Finding[] {
  const map = new Map<string, Finding>();
  for (const r of results) {
    for (const s of r.subresults) {
      const key = JSON.stringify([s.severity, s.code ?? null, s.message ?? "", signatureExtra(s)]);
      if (!map.has(key)) {
        map.set(key, {
          key,
          severity: s.severity,
          code: s.code ?? null,
          message: s.message ?? "",
          status: s,
          files: [],
        });
      }
      const f = map.get(key)!;
      if (!f.files.includes(fileOf(r))) f.files.push(fileOf(r));
    }
  }
  return [...map.values()].sort(
    (a, b) => severityRank(a.severity) - severityRank(b.severity) || b.files.length - a.files.length,
  );
}
