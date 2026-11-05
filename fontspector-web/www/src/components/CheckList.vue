<script setup lang="ts">
import { reactive } from 'vue';
import { STATUS_LABELS } from '../constants';
import { CheckGroup, CheckSummary, FAMILY, fileOf, filters, fontCount, resetFilters, selection } from '../results';
import { isSelected } from '../fixes';

defineProps<{ groups: CheckGroup[]; total: number; visible: number }>();

const collapsed = reactive(new Set<string>());
function toggleGroup(key: string) {
  collapsed.has(key) ? collapsed.delete(key) : collapsed.add(key);
}

function fileLabel(c: CheckSummary): string {
  const n = fontCount();
  if (filters.groupBy === 'file' || n < 2 || c.results.every((r) => fileOf(r) === FAMILY)) return '';
  const bad = c.results.filter((r) => r.worst_status === c.worst).length;
  return bad === n ? `all ${n} files` : `${bad} of ${n} files`;
}

function hasSelectedFix(c: CheckSummary) {
  return c.results.some(isSelected);
}
</script>

<template>
  <div class="checklist">
    <button type="button" class="check-row overview-row" :class="{ 'is-selected': selection.checkId === null }"
      @click="selection.checkId = null">
      <span class="overview-icon">◎</span>
      <span class="check-name">Overview</span>
    </button>

    <div class="list-meta">
      Showing {{ visible }} of {{ total }} checks
    </div>

    <div v-if="groups.length === 0" class="empty">
      <p>No checks match these filters.</p>
      <button class="btn btn-sm btn-outline-secondary" @click="resetFilters">Reset filters</button>
    </div>

    <div v-for="g in groups" :key="g.key" class="group">
      <button type="button" class="group-head" :class="g.severity && `sev-${g.severity}`"
        @click="toggleGroup(g.key)" :aria-expanded="!collapsed.has(g.key)">
        <span class="chev" :class="{ closed: collapsed.has(g.key) }">▾</span>
        <span v-if="g.severity" class="sev-dot"></span>
        <span class="group-label" :title="g.key">{{ g.severity ? STATUS_LABELS[g.severity] : g.label }}</span>
        <span class="group-count">{{ g.checks.length }}</span>
      </button>
      <template v-if="!collapsed.has(g.key)">
        <button v-for="c in g.checks" :key="c.id" type="button" class="check-row" :class="[`sev-${c.worst}`,
          { 'is-selected': selection.checkId === c.id }]" @click="selection.checkId = c.id">
          <span class="sev-bar"></span>
          <span class="check-body">
            <span class="check-name">{{ c.name }}</span>
            <span class="check-meta">
              <code>{{ c.id }}</code>
              <span v-if="fileLabel(c)">· {{ fileLabel(c) }}</span>
            </span>
          </span>
          <span class="check-tags">
            <span v-if="!g.severity" class="sev-pill">{{ c.worst }}</span>
            <span v-if="c.fixable" class="fix-tag" :class="{ chosen: hasSelectedFix(c) }"
              :title="hasSelectedFix(c) ? 'Fix selected' : 'Auto-fixable'">🔧</span>
          </span>
        </button>
      </template>
    </div>
  </div>
</template>

<style>
.checklist {
  padding-bottom: 2rem;
}

.list-meta {
  font-size: .75rem;
  color: var(--bs-secondary-color);
  padding: .5rem 1rem;
  border-bottom: 1px solid var(--bs-border-color);
}

.empty {
  padding: 2rem 1rem;
  text-align: center;
  color: var(--bs-secondary-color);
}

.group-head {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: .5rem;
  width: 100%;
  border: 0;
  border-bottom: 1px solid var(--bs-border-color);
  padding: .45rem 1rem;
  background: var(--bs-secondary-bg);
  color: var(--bs-body-color);
  font-size: .8rem;
  font-weight: 600;
  text-align: left;
}

.group-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-count {
  font-variant-numeric: tabular-nums;
  color: var(--bs-secondary-color);
  font-weight: normal;
}

.chev {
  display: inline-block;
  transition: transform .15s;
  width: .8rem;
}

.chev.closed {
  transform: rotate(-90deg);
}

.check-row {
  display: flex;
  align-items: stretch;
  gap: .6rem;
  width: 100%;
  border: 0;
  border-bottom: 1px solid var(--bs-border-color-translucent);
  background: none;
  color: var(--bs-body-color);
  padding: .55rem 1rem .55rem 0;
  text-align: left;
}

.check-row:hover {
  background: var(--bs-tertiary-bg);
}

.check-row.is-selected {
  background: color-mix(in srgb, var(--bs-primary) 12%, var(--bs-body-bg));
}

.sev-bar {
  width: 4px;
  flex: none;
  border-radius: 0 2px 2px 0;
  background: var(--sev);
}

.check-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: .15rem;
}

.check-name {
  font-size: .875rem;
  line-height: 1.3;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.check-meta {
  font-size: .72rem;
  color: var(--bs-secondary-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.check-meta code {
  color: inherit;
}

.check-tags {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: .2rem;
}

.fix-tag {
  font-size: .8rem;
  opacity: .55;
  filter: grayscale(1);
}

.fix-tag.chosen {
  opacity: 1;
  filter: none;
}

.overview-row {
  padding-left: 1rem;
  align-items: center;
  font-weight: 600;
}

.overview-icon {
  color: var(--bs-primary);
}
</style>
