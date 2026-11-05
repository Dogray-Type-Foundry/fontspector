<script setup lang="ts">
import { computed, ref } from 'vue';
import { state } from '../store';
import { renderMarkdown } from '../markdown';
import { CheckSpecificRendering } from '../rendering';
import { baseName, CheckSummary, collateFindings, fileOf, Finding, fontCount, isFixable } from '../results';
import { canSelect, dialogRequestFor, getReplies, isSelected, setReplies, setSelected, setValid } from '../fixes';
import { CheckResult } from '../types';
import FixDialog from './FixDialog.vue';

const props = defineProps<{ check: CheckSummary }>();

const renderer = computed(() => CheckSpecificRendering[props.check.id] as typeof CheckSpecificRendering[string] | undefined);
const findings = computed(() =>
  collateFindings(props.check.results, (s) => (renderer.value ? JSON.stringify(s.metadata ?? null) : '')));

function findingHtml(f: Finding): string {
  let html = renderMarkdown(f.message);
  if (renderer.value && f.status.metadata && state.fonts[f.files[0]]) {
    const [extra, suppress] = renderer.value(f.status.metadata, state.fonts[f.files[0]]);
    html = suppress ? extra : html + extra;
  }
  return html;
}

const multiFile = computed(() => props.check.results.length > 1);
const nFonts = computed(() => Math.max(fontCount(), props.check.results.length));
const docs = computed(() => state.allChecks[props.check.id]?.proposal || []);
const fixable = computed(() => props.check.results.filter(isFixable));
const sortedResults = computed(() =>
  [...props.check.results].sort((a, b) => fileOf(a).localeCompare(fileOf(b))));

// One dialog per distinct "more information" request among the fixable files.
const dialogGroups = computed(() => {
  const groups = new Map<string, CheckResult[]>();
  for (const r of fixable.value) {
    const req = dialogRequestFor(r);
    if (!req) continue;
    const k = JSON.stringify(req);
    if (!groups.has(k)) groups.set(k, []);
    groups.get(k)!.push(r);
  }
  return [...groups.values()].map((rs) => ({ request: dialogRequestFor(rs[0])!, results: rs }));
});

const allFixesSelected = computed(() => fixable.value.length > 0 && fixable.value.every(isSelected));
function selectAllFixes(on: boolean) {
  for (const r of fixable.value) setSelected(r, on);
}

const copied = ref(false);
function copyId() {
  navigator.clipboard?.writeText(props.check.id);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1200);
}
</script>

<template>
  <article class="detail">
    <header class="detail-head">
      <div class="d-flex align-items-center gap-2 flex-wrap mb-2">
        <span class="sev-pill" :class="`sev-${check.worst}`">{{ check.worst }}</span>
        <span class="text-body-secondary small">{{ check.section }}</span>
        <span v-if="fixable.length" class="fix-badge">🔧 Auto-fixable</span>
      </div>
      <h2>{{ check.name }}</h2>
      <div class="d-flex align-items-center gap-3 flex-wrap small">
        <button class="id-btn" @click="copyId" title="Copy check ID"><code>{{ check.id }}</code>
          <span>{{ copied ? '✓ copied' : '⧉' }}</span></button>
        <a v-for="(url, i) in docs" :key="url" :href="url" target="_blank" rel="noopener">
          More info{{ docs.length > 1 ? ` ${i + 1}` : '' }} ↗</a>
      </div>
    </header>

    <details class="rationale-box" open>
      <summary>Why this matters</summary>
      <div class="md" v-html="renderMarkdown(check.rationale)"></div>
    </details>

    <section>
      <h3>Findings <span class="count">{{ findings.length }}</span></h3>
      <div v-for="f in findings" :key="f.key" class="finding" :class="`sev-${f.severity}`">
        <div class="finding-head">
          <span class="sev-pill">{{ f.severity }}</span>
          <code v-if="f.code" class="finding-code">{{ f.code }}</code>
          <span v-if="multiFile" class="finding-files">
            <template v-if="f.files.length === nFonts">All {{ f.files.length }} files</template>
            <template v-else>{{ f.files.length }} of {{ nFonts }} files</template>
          </span>
        </div>
        <div class="md" v-html="findingHtml(f)"></div>
        <div v-if="multiFile && f.files.length < nFonts" class="file-chips">
          <span v-for="file in f.files" :key="file" class="file-chip" :title="file">{{ baseName(file) }}</span>
        </div>
      </div>
    </section>

    <section v-if="multiFile || fixable.length">
      <h3>Files
        <label v-if="fixable.length > 1" class="select-all">
          <input type="checkbox" class="form-check-input" :checked="allFixesSelected"
            @change="selectAllFixes(($event.target as HTMLInputElement).checked)" />
          Fix all {{ fixable.length }}
        </label>
      </h3>
      <table class="file-table">
        <tbody>
          <tr v-for="r in sortedResults" :key="fileOf(r)">
            <td :title="fileOf(r)">{{ baseName(fileOf(r)) }}</td>
            <td><span class="sev-pill" :class="`sev-${r.worst_status}`">{{ r.worst_status }}</span></td>
            <td class="text-end">
              <label v-if="isFixable(r)" class="fix-check" :class="{ disabled: !canSelect(r) }"
                :title="canSelect(r) ? 'Include in fixes' : 'Answer the questions below first'">
                <input type="checkbox" class="form-check-input" :checked="isSelected(r)" :disabled="!canSelect(r)"
                  @change="setSelected(r, ($event.target as HTMLInputElement).checked)" /> Fix
              </label>
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <FixDialog v-for="(g, i) in dialogGroups" :key="i" :request="g.request" :model-value="getReplies(g.results)"
      @update:model-value="(v) => setReplies(g.results, v)" @update:valid="(v) => setValid(g.results, v)" />
  </article>
</template>

<style>
.detail {
  padding: 1.5rem 2rem 2rem;
  max-width: 960px;
  width: 100%;
}

.detail-head h2 {
  font-size: 1.35rem;
  font-weight: 600;
  margin-bottom: .4rem;
}

.detail h3 {
  font-size: .75rem;
  text-transform: uppercase;
  letter-spacing: .06em;
  color: var(--bs-secondary-color);
  margin: 1.75rem 0 .75rem;
  display: flex;
  align-items: center;
  gap: .5rem;
}

.detail h3 .count {
  font-variant-numeric: tabular-nums;
}

.fix-badge {
  font-size: .72rem;
  padding: .1em .55em;
  border-radius: 999px;
  border: 1px solid var(--bs-border-color);
  color: var(--bs-secondary-color);
}

.id-btn {
  border: 0;
  background: var(--bs-secondary-bg);
  border-radius: .3rem;
  padding: .1rem .45rem;
  display: inline-flex;
  gap: .4rem;
  color: var(--bs-secondary-color);
}

.rationale-box {
  margin-top: 1.25rem;
  background: var(--bs-tertiary-bg);
  border: 1px solid var(--bs-border-color);
  border-radius: .5rem;
  padding: .6rem 1rem;
}

.rationale-box summary {
  font-size: .8rem;
  font-weight: 600;
  color: var(--bs-secondary-color);
  cursor: pointer;
}

.rationale-box[open] summary {
  margin-bottom: .5rem;
}

.md {
  font-size: .9rem;
  overflow-wrap: anywhere;
}

.md> :last-child {
  margin-bottom: 0;
}

.md pre {
  background: var(--bs-secondary-bg);
  padding: .5rem .75rem;
  border-radius: .3rem;
}

.md table {
  font-size: .82rem;
}

.md th {
  white-space: nowrap;
}

.finding {
  border: 1px solid var(--bs-border-color);
  border-left: 4px solid var(--sev);
  border-radius: .4rem;
  padding: .75rem 1rem;
  margin-bottom: .6rem;
}

.finding-head {
  display: flex;
  align-items: center;
  gap: .6rem;
  margin-bottom: .4rem;
}

.finding-code {
  font-size: .75rem;
  color: var(--bs-secondary-color);
}

.finding-files {
  margin-left: auto;
  font-size: .75rem;
  color: var(--bs-secondary-color);
}

.file-chips {
  display: flex;
  flex-wrap: wrap;
  gap: .3rem;
  margin-top: .5rem;
}

.file-chip {
  font-size: .72rem;
  padding: .05rem .45rem;
  border-radius: .3rem;
  background: var(--bs-secondary-bg);
  color: var(--bs-secondary-color);
}

.file-table {
  width: 100%;
  margin: 0;
  background: none;
  font-size: .85rem;
}

.file-table td {
  background: none !important;
  border: 0 !important;
  border-bottom: 1px solid var(--bs-border-color-translucent) !important;
  padding: .35rem .25rem;
}

.file-table tr {
  background: none !important;
}

.select-all,
.fix-check {
  display: inline-flex;
  align-items: center;
  gap: .35rem;
  text-transform: none;
  letter-spacing: 0;
  font-size: .8rem;
  color: var(--bs-body-color);
  cursor: pointer;
  margin: 0;
}

.select-all {
  margin-left: auto;
}

.fix-check.disabled {
  opacity: .5;
  cursor: not-allowed;
}
</style>
