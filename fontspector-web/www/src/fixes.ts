// Shared hotfix selection state, so selections survive moving between checks.
import { reactive } from "vue";
import { postToWorker } from "./workersingleton";
import {
  CheckResult,
  FixItem,
  isFixNeedsMoreInformation,
  MoreInfoReply,
  MoreInfoRequest,
} from "./types";
import { isFixable } from "./results";

export const fixes = reactive({
  selected: new Map<string, FixItem>(),
  replies: new Map<string, MoreInfoReply>(),
  validity: new Map<string, boolean>(),
});

function key(r: CheckResult): string {
  return `${r.check_id}::${r.filename}`;
}

export function dialogRequestFor(r: CheckResult): MoreInfoRequest | null {
  for (const s of r.subresults) {
    for (const m of s.metadata || []) {
      if (isFixNeedsMoreInformation(m)) return m.FixNeedsMoreInformation;
    }
  }
  return null;
}

export function canSelect(r: CheckResult): boolean {
  return isFixable(r) && (!dialogRequestFor(r) || fixes.validity.get(key(r)) === true);
}

export function isSelected(r: CheckResult): boolean {
  return fixes.selected.has(key(r));
}

export function setSelected(r: CheckResult, on: boolean) {
  if (on && canSelect(r)) {
    const replies = fixes.replies.get(key(r));
    fixes.selected.set(key(r), {
      check_id: r.check_id,
      filename: r.filename!,
      details: replies ? { ...replies } : null,
    });
  } else {
    fixes.selected.delete(key(r));
  }
}

export function getReplies(results: CheckResult[]): MoreInfoReply {
  return (results.length && fixes.replies.get(key(results[0]))) || {};
}

export function setReplies(results: CheckResult[], replies: MoreInfoReply) {
  for (const r of results) {
    fixes.replies.set(key(r), { ...replies });
    const sel = fixes.selected.get(key(r));
    if (sel) fixes.selected.set(key(r), { ...sel, details: { ...replies } });
  }
}

export function setValid(results: CheckResult[], valid: boolean) {
  for (const r of results) {
    fixes.validity.set(key(r), valid);
    if (!valid) fixes.selected.delete(key(r));
  }
}

export function applyFixes(download: boolean) {
  const requests: FixItem[] = [...fixes.selected.values()].map((i) => ({
    // Deproxify before posting to the worker
    check_id: i.check_id,
    filename: i.filename,
    details: i.details ? { ...i.details } : null,
  }));
  postToWorker({ id: "fix", requests, download });
  fixes.selected.clear();
  fixes.replies.clear();
  fixes.validity.clear();
}
