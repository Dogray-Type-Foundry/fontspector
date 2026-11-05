<script setup lang="ts">
import { computed } from 'vue';
import { PROFILES, STATUS_LABELS, STATUS_ORDER } from '../constants';
import { state } from '../store';
import { CheckResult, StatusCode } from '../types';
import {
  aggregate, baseName, countBySeverity, fileOf, filters, fontCount, ISSUE_LEVELS, resetFilters, sectionOf,
} from '../results';
import { canSelect, setSelected } from '../fixes';

const props = defineProps<{ results: CheckResult[] }>();

const checks = computed(() => aggregate(props.results));
const counts = computed(() => countBySeverity(checks.value));
const total = computed(() => checks.value.length);

const verdict = computed(() => {
  const c = counts.value;
  const blocking = c.FATAL + c.ERROR + c.FAIL;
  if (blocking) return { tone: 'FAIL', text: `${blocking} check${blocking === 1 ? '' : 's'} need attention` };
  if (c.WARN) return { tone: 'WARN', text: `No failures — ${c.WARN} warning${c.WARN === 1 ? '' : 's'} to review` };
  return { tone: 'PASS', text: 'All checks passed' };
});

type Row = { key: string; label: string; counts: Record<StatusCode, number> };
function matrix(keyOf: (r: CheckResult) => string): Row[] {
  const rows = new Map<string, CheckResult[]>();
  for (const r of props.results) {
    const k = keyOf(r);
    if (!rows.has(k)) rows.set(k, []);
    rows.get(k)!.push(r);
  }
  return [...rows.entries()]
    .map(([key, rs]) => ({
      key,
      label: keyOf === fileOf ? baseName(key) : key,
      counts: countBySeverity(aggregate(rs)),
    }))
    .sort((a, b) => a.key.localeCompare(b.key));
}
const tables = computed(() => {
  const files = matrix(fileOf);
  return [
    ...(files.length > 1 ? [{ title: 'By file', kind: 'file' as const, rows: files }] : []),
    { title: 'By section', kind: 'section' as const, rows: matrix(sectionOf) },
  ];
});
const COLS: StatusCode[] = [...ISSUE_LEVELS, 'INFO'];

function show(opts: { file?: string; section?: string; severity?: StatusCode }) {
  resetFilters();
  if (opts.file) filters.files.add(opts.file);
  if (opts.section) filters.sections.add(opts.section);
  if (opts.severity) filters.severities = new Set([opts.severity]);
}

const fixableResults = computed(() => props.results.filter(canSelect));
const fixableChecks = computed(() => new Set(fixableResults.value.map((r) => r.check_id)).size);
function selectAllFixes() {
  for (const r of fixableResults.value) setSelected(r, true);
}
</script>

<template>
  <div class="summary">
    <p class="eyebrow">{{ PROFILES[state.selectedProfile] }} · {{ fontCount() }} font{{ fontCount() === 1 ? '' : 's' }}
      · {{ total }} checks</p>
    <h2 :class="`sev-${verdict.tone}`"><span class="sev-dot"></span> {{ verdict.text }}</h2>

    <div class="sev-bar-wide" role="img" :aria-label="STATUS_ORDER.map(s => `${counts[s]} ${s}`).join(', ')">
      <span v-for="s in STATUS_ORDER" :key="s" v-show="counts[s]" :class="`sev-${s}`"
        :style="{ flexGrow: counts[s] }" :title="`${counts[s]} ${STATUS_LABELS[s]}`"></span>
    </div>

    <div class="tiles">
      <button v-for="s in STATUS_ORDER" :key="s" type="button" class="tile" :class="`sev-${s}`"
        :disabled="!counts[s]" @click="show({ severity: s })">
        <span class="tile-n">{{ counts[s] }}</span>
        <span class="tile-l"><span class="sev-dot"></span> {{ STATUS_LABELS[s] }}</span>
      </button>
    </div>

    <div v-if="fixableResults.length" class="fix-callout">
      <span>🔧 <b>{{ fixableResults.length }}</b> issue{{ fixableResults.length === 1 ? '' : 's' }} across
        {{ fixableChecks }} check{{ fixableChecks === 1 ? '' : 's' }} can be fixed automatically.</span>
      <span class="ms-auto d-flex gap-2">
        <button class="btn btn-sm btn-link" @click="show({}); filters.fixableOnly = true">Review</button>
        <button class="btn btn-sm btn-primary" @click="selectAllFixes">Select all fixes</button>
      </span>
    </div>

    <template v-for="t in tables" :key="t.title">
      <h3>{{ t.title }} <small>checks by worst result — click to filter</small></h3>
      <table class="matrix">
        <thead>
          <tr>
            <th></th>
            <th v-for="s in COLS" :key="s" :class="`sev-${s}`"><span class="sev-dot"></span> {{ STATUS_LABELS[s] }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in t.rows" :key="row.key" @click="show({ [t.kind]: row.key })" :title="row.key">
            <td class="row-label">{{ row.label }}</td>
            <td v-for="s in COLS" :key="s" :class="[`sev-${s}`, { zero: !row.counts[s] }]">
              <span class="cell">{{ row.counts[s] || '·' }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </template>
  </div>
</template>

<style>
.summary {
  padding: 1.75rem 2rem 2rem;
  max-width: 960px;
}

.summary .eyebrow {
  font-size: .8rem;
  color: var(--bs-secondary-color);
  margin-bottom: .25rem;
}

.summary h2 {
  font-size: 1.6rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: .6rem;
}

.summary h2 .sev-dot {
  width: .85rem;
  height: .85rem;
}

.sev-bar-wide {
  display: flex;
  height: 10px;
  border-radius: 5px;
  overflow: hidden;
  gap: 2px;
  margin: 1.25rem 0 1rem;
}

.sev-bar-wide span {
  background: var(--sev);
  min-width: 4px;
}

.tiles {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(92px, 1fr));
  gap: .6rem;
}

.tile {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  border: 1px solid var(--bs-border-color);
  border-radius: .5rem;
  background: var(--bs-body-bg);
  color: var(--bs-body-color);
  padding: .6rem .8rem;
  text-align: left;
}

.tile:not(:disabled):hover {
  border-color: var(--sev);
  background: color-mix(in srgb, var(--sev) 7%, var(--bs-body-bg));
}

.tile:disabled {
  opacity: .45;
}

.tile-n {
  font-size: 1.5rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  line-height: 1.2;
}

.tile-l {
  font-size: .78rem;
  color: var(--bs-secondary-color);
  display: flex;
  align-items: center;
  gap: .35rem;
}

.fix-callout {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: .75rem;
  margin-top: 1.25rem;
  padding: .75rem 1rem;
  border-radius: .5rem;
  background: color-mix(in srgb, var(--bs-primary) 9%, var(--bs-body-bg));
  border: 1px solid color-mix(in srgb, var(--bs-primary) 25%, transparent);
  font-size: .9rem;
}

.summary h3 {
  font-size: .75rem;
  text-transform: uppercase;
  letter-spacing: .06em;
  color: var(--bs-secondary-color);
  margin: 2rem 0 .5rem;
}

.summary h3 small {
  text-transform: none;
  letter-spacing: 0;
  font-weight: normal;
  margin-left: .5rem;
}

.matrix {
  width: 100%;
  margin: 0;
  background: none;
  font-size: .85rem;
  border-collapse: collapse;
}

.matrix th,
.matrix td {
  background: none !important;
  border: 0 !important;
  border-bottom: 1px solid var(--bs-border-color-translucent) !important;
  padding: .35rem .5rem;
  text-align: center;
  color: var(--bs-body-color);
  width: 4.5rem;
}

.matrix th {
  font-size: .72rem;
  font-weight: 600;
  color: var(--bs-secondary-color);
  white-space: nowrap;
}

.matrix tr {
  background: none !important;
  cursor: pointer;
}

.matrix tbody tr:hover td {
  background: var(--bs-tertiary-bg) !important;
}

.matrix .row-label {
  text-align: left;
  width: auto;
  max-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.matrix .cell {
  display: inline-block;
  min-width: 1.8rem;
  border-radius: .3rem;
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  color: var(--sev);
  background: color-mix(in srgb, var(--sev) 13%, transparent);
}

.matrix .zero .cell {
  background: none;
  color: var(--bs-secondary-color);
  font-weight: normal;
  opacity: .5;
}
</style>
