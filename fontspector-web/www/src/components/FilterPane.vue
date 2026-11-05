<script setup lang="ts">
import { computed, ref } from 'vue';
import { STATUS_LABELS, STATUS_ORDER } from '../constants';
import { CheckResult, StatusCode } from '../types';
import {
  baseName, CheckSummary, countBySeverity, fileOf, filters, GroupBy, ISSUE_LEVELS, resetFilters, sectionOf,
} from '../results';

const props = defineProps<{ results: CheckResult[]; preSeverity: CheckSummary[] }>();

const search = ref<HTMLInputElement | null>(null);
defineExpose({ focusSearch: () => search.value?.focus() });

const sevCounts = computed(() => countBySeverity(props.preSeverity));

// Issues (WARN or worse) per file / per section, independent of other filters.
function issueCounts(keyOf: (r: CheckResult) => string) {
  const counts = new Map<string, number>();
  for (const r of props.results) {
    const k = keyOf(r);
    counts.set(k, (counts.get(k) || 0) + (ISSUE_LEVELS.includes(r.worst_status) ? 1 : 0));
  }
  return [...counts.entries()].sort(([a], [b]) => a.localeCompare(b));
}
const files = computed(() => issueCounts(fileOf));
const sections = computed(() => {
  // Count unique checks, not check × file
  const seen = new Map<string, Set<string>>();
  for (const r of props.results) {
    if (!ISSUE_LEVELS.includes(r.worst_status)) continue;
    const s = sectionOf(r);
    if (!seen.has(s)) seen.set(s, new Set());
    seen.get(s)!.add(r.check_id);
  }
  const all = new Set(props.results.map(sectionOf));
  return [...all].sort().map((s) => [s, seen.get(s)?.size || 0] as [string, number]);
});

function toggle<T>(set: Set<T>, v: T) {
  set.has(v) ? set.delete(v) : set.add(v);
}

function soloSeverity(s: StatusCode) {
  filters.severities = new Set([s]);
}

const groupOptions: [GroupBy, string][] = [
  ['severity', 'Severity'], ['section', 'Section'], ['file', 'File'], ['none', 'None'],
];

const isDefault = computed(() =>
  !filters.search && !filters.fixableOnly && !filters.files.size && !filters.sections.size &&
  filters.severities.size === 5 && !filters.severities.has('PASS') && !filters.severities.has('SKIP'));
</script>

<template>
  <div class="filters">
    <div class="search-wrap">
      <input ref="search" v-model="filters.search" type="search" class="form-control form-control-sm"
        placeholder="Search checks & messages" aria-label="Search" />
      <kbd class="search-hint">/</kbd>
    </div>

    <section>
      <h6>Severity <small>double-click to isolate</small></h6>
      <button v-for="s in STATUS_ORDER" :key="s" type="button" class="sev-toggle" :class="[`sev-${s}`,
        { on: filters.severities.has(s) }]" @click="toggle(filters.severities, s)" @dblclick="soloSeverity(s)">
        <span class="sev-dot"></span>
        <span class="flex-grow-1 text-start">{{ STATUS_LABELS[s] }}</span>
        <span class="count">{{ sevCounts[s] }}</span>
      </button>
    </section>

    <section>
      <label class="form-check form-switch">
        <input v-model="filters.fixableOnly" class="form-check-input" type="checkbox" />
        <span class="form-check-label">Auto-fixable only</span>
      </label>
    </section>

    <section>
      <h6>Group by</h6>
      <div class="btn-group btn-group-sm w-100" role="group">
        <template v-for="[v, label] in groupOptions" :key="v">
          <input :id="`group-${v}`" v-model="filters.groupBy" type="radio" class="btn-check" :value="v" />
          <label class="btn btn-outline-secondary" :for="`group-${v}`">{{ label }}</label>
        </template>
      </div>
    </section>

    <section v-if="files.length > 1">
      <h6>Files <small v-if="filters.files.size">
          <a href="#" @click.prevent="filters.files.clear()">all</a></small></h6>
      <label v-for="[f, n] in files" :key="f" class="facet" :title="f">
        <input type="checkbox" class="form-check-input" :checked="filters.files.has(f)"
          @change="toggle(filters.files, f)" />
        <span class="facet-name">{{ baseName(f) }}</span>
        <span class="count" title="Checks at WARN or worse">{{ n }}</span>
      </label>
    </section>

    <section>
      <h6>Sections <small v-if="filters.sections.size">
          <a href="#" @click.prevent="filters.sections.clear()">all</a></small></h6>
      <label v-for="[s, n] in sections" :key="s" class="facet" :title="s">
        <input type="checkbox" class="form-check-input" :checked="filters.sections.has(s)"
          @change="toggle(filters.sections, s)" />
        <span class="facet-name">{{ s }}</span>
        <span class="count">{{ n }}</span>
      </label>
    </section>

    <button v-if="!isDefault" class="btn btn-sm btn-outline-secondary w-100" @click="resetFilters">
      Reset filters
    </button>

    <p class="shortcuts">
      <kbd>j</kbd>/<kbd>k</kbd> next/prev · <kbd>/</kbd> search · <kbd>Esc</kbd> overview
    </p>
  </div>
</template>

<style>
.filters section {
  margin: 1.1rem 0;
}

.filters h6 {
  font-size: .7rem;
  text-transform: uppercase;
  letter-spacing: .06em;
  color: var(--bs-secondary-color);
  margin-bottom: .4rem;
  display: flex;
  justify-content: space-between;
}

.filters h6 small {
  text-transform: none;
  letter-spacing: 0;
  font-weight: normal;
}

.search-wrap {
  position: relative;
}

.search-hint {
  position: absolute;
  right: .5rem;
  top: 50%;
  transform: translateY(-50%);
  pointer-events: none;
  opacity: .6;
}

.search-wrap input:focus+.search-hint,
.search-wrap input:not(:placeholder-shown)+.search-hint {
  display: none;
}

.filters kbd {
  font-size: .65rem;
  padding: .05rem .3rem;
  background: var(--bs-secondary-bg);
  color: var(--bs-body-color);
  border: 1px solid var(--bs-border-color);
}

.sev-toggle {
  display: flex;
  align-items: center;
  gap: .5rem;
  width: 100%;
  border: 0;
  background: none;
  padding: .3rem .5rem;
  border-radius: .4rem;
  color: var(--bs-body-color);
  font-size: .875rem;
  opacity: .45;
  user-select: none;
}

.sev-toggle.on {
  opacity: 1;
  background: color-mix(in srgb, var(--sev) 12%, transparent);
}

.sev-toggle:hover {
  background: color-mix(in srgb, var(--sev) 18%, transparent);
}

.filters .count {
  font-variant-numeric: tabular-nums;
  font-size: .8rem;
  color: var(--bs-secondary-color);
}

.facet {
  display: flex;
  align-items: center;
  gap: .5rem;
  padding: .15rem .25rem;
  font-size: .85rem;
  cursor: pointer;
  border-radius: .3rem;
}

.facet:hover {
  background: var(--bs-secondary-bg);
}

.facet .form-check-input {
  margin: 0;
  flex: none;
}

.facet-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shortcuts {
  margin-top: 1.5rem;
  font-size: .75rem;
  color: var(--bs-secondary-color);
}
</style>
