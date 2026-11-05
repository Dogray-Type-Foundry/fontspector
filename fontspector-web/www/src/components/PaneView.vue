<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { state } from '../store';
import { applyNonSeverityFilters, filters, groupChecks, selection } from '../results';
import { applyFixes, fixes } from '../fixes';
import FilterPane from './FilterPane.vue';
import CheckList from './CheckList.vue';
import CheckDetail from './CheckDetail.vue';
import SummaryPanel from './SummaryPanel.vue';

const results = computed(() => state.lastResults || []);
const preSeverity = computed(() => applyNonSeverityFilters(results.value));
const visible = computed(() => preSeverity.value.filter((c) => filters.severities.has(c.worst)));
const groups = computed(() => groupChecks(visible.value, filters.groupBy));
const selected = computed(() => preSeverity.value.find((c) => c.id === selection.checkId) || null);

// Flat order of what's on screen, for keyboard navigation.
const order = computed(() => {
  const ids: (string | null)[] = [null];
  for (const g of groups.value) for (const c of g.checks) if (!ids.includes(c.id)) ids.push(c.id);
  return ids;
});

const listPane = ref<HTMLElement | null>(null);
const detailPane = ref<HTMLElement | null>(null);
const filterPane = ref<InstanceType<typeof FilterPane> | null>(null);

function move(delta: number) {
  const i = order.value.indexOf(selection.checkId);
  const next = order.value[Math.max(0, Math.min(order.value.length - 1, i + delta))];
  selection.checkId = next ?? null;
  nextTick(() => listPane.value?.querySelector('.is-selected')?.scrollIntoView({ block: 'nearest' }));
}

function onKey(e: KeyboardEvent) {
  const t = e.target as HTMLElement;
  if (t.closest('input:not([type=checkbox]):not([type=radio]), textarea, select')) {
    if (e.key === 'Escape') (t as HTMLElement).blur();
    return;
  }
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  if (e.key === 'j' || e.key === 'ArrowDown') { move(1); e.preventDefault(); }
  else if (e.key === 'k' || e.key === 'ArrowUp') { move(-1); e.preventDefault(); }
  else if (e.key === '/') { filterPane.value?.focusSearch(); e.preventDefault(); }
  else if (e.key === 'Escape') selection.checkId = null;
}

watch(() => selection.checkId, () => detailPane.value?.scrollTo({ top: 0 }));
onMounted(() => window.addEventListener('keydown', onKey));
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));

// Resizable pane widths, remembered per browser.
const DEFAULT_WIDTHS = { filters: 290, list: 380 };
const LIMITS = { filters: [200, 520], list: [260, 760] } as const;
type PaneName = keyof typeof DEFAULT_WIDTHS;
function loadWidths(): Record<PaneName, number> {
  try {
    return { ...DEFAULT_WIDTHS, ...JSON.parse(localStorage.getItem('fontspector-pane-widths') || '{}') };
  } catch {
    return { ...DEFAULT_WIDTHS };
  }
}
const widths = ref(loadWidths());
function saveWidths() {
  try { localStorage.setItem('fontspector-pane-widths', JSON.stringify(widths.value)); } catch { /* ignore */ }
}
function startResize(pane: PaneName, e: PointerEvent) {
  const handle = e.currentTarget as HTMLElement;
  const startX = e.clientX;
  const startW = widths.value[pane];
  const [min, limit] = LIMITS[pane];
  // Always leave the detail pane at least 420px.
  const other = pane === 'filters' ? widths.value.list : widths.value.filters;
  const max = Math.max(min, Math.min(limit, window.innerWidth - other - 420));
  handle.setPointerCapture(e.pointerId);
  const onMove = (ev: PointerEvent) => {
    widths.value[pane] = Math.round(Math.min(max, Math.max(min, startW + ev.clientX - startX)));
  };
  const onUp = () => {
    handle.removeEventListener('pointermove', onMove);
    handle.removeEventListener('pointerup', onUp);
    document.body.classList.remove('resizing');
    saveWidths();
  };
  handle.addEventListener('pointermove', onMove);
  handle.addEventListener('pointerup', onUp);
  document.body.classList.add('resizing');
}
function resetWidth(pane: PaneName) {
  widths.value[pane] = DEFAULT_WIDTHS[pane];
  saveWidths();
}

const fixFileCount = computed(() => new Set([...fixes.selected.values()].map((f) => f.filename)).size);
</script>

<template>
  <div class="report" :style="{ '--w-filters': `${widths.filters}px`, '--w-list': `${widths.list}px` }">
    <aside class="pane pane-filters">
      <FilterPane ref="filterPane" :results="results" :pre-severity="preSeverity" />
    </aside>
    <div class="resizer" role="separator" aria-orientation="vertical" title="Drag to resize · double-click to reset"
      @pointerdown.prevent="startResize('filters', $event)" @dblclick="resetWidth('filters')"></div>

    <nav class="pane pane-list" ref="listPane" aria-label="Checks">
      <CheckList :groups="groups" :total="preSeverity.length" :visible="visible.length" />
    </nav>
    <div class="resizer" role="separator" aria-orientation="vertical" title="Drag to resize · double-click to reset"
      @pointerdown.prevent="startResize('list', $event)" @dblclick="resetWidth('list')"></div>

    <main class="pane pane-detail" ref="detailPane">
      <CheckDetail v-if="selected" :check="selected" />
      <SummaryPanel v-else :results="results" />

      <div v-if="fixes.selected.size" class="fix-tray">
        <span><b>{{ fixes.selected.size }}</b> fix{{ fixes.selected.size === 1 ? '' : 'es' }} selected
          across {{ fixFileCount }} file{{ fixFileCount === 1 ? '' : 's' }}</span>
        <span class="ms-auto d-flex gap-2">
          <button class="btn btn-sm btn-link" @click="fixes.selected.clear()">Clear</button>
          <button class="btn btn-sm btn-outline-primary" @click="applyFixes(false)"
            title="Apply fixes and re-run checks">Apply &amp; re-check</button>
          <button class="btn btn-sm btn-primary" @click="applyFixes(true)">Apply &amp; download</button>
        </span>
      </div>
    </main>
  </div>
</template>

<style>
.report {
  display: grid;
  grid-template-columns: var(--w-filters) 0 var(--w-list) 0 minmax(0, 1fr);
  flex: 1;
  min-height: 0;
  border-top: 1px solid var(--bs-border-color);
}

.report .pane {
  overflow-y: auto;
  min-height: 0;
  overscroll-behavior: contain;
}

.resizer {
  position: relative;
  z-index: 2;
  cursor: col-resize;
  touch-action: none;
}

/* Wide invisible hit area, with a highlight line on hover/drag */
.resizer::before {
  content: "";
  position: absolute;
  inset: 0 -4px;
}

.resizer:hover::after,
body.resizing .resizer:active::after {
  content: "";
  position: absolute;
  top: 0;
  bottom: 0;
  left: -1px;
  width: 3px;
  background: var(--bs-primary);
  opacity: .6;
}

body.resizing {
  cursor: col-resize;
  user-select: none;
}

.pane-filters {
  background: var(--bs-tertiary-bg);
  border-right: 1px solid var(--bs-border-color);
  padding: 1rem;
}

.pane-list {
  border-right: 1px solid var(--bs-border-color);
}

.pane-detail {
  position: relative;
  display: flex;
  flex-direction: column;
}

.fix-tray {
  position: sticky;
  bottom: 0;
  margin-top: auto;
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: .6rem 1.5rem;
  background: color-mix(in srgb, var(--bs-primary) 10%, var(--bs-body-bg));
  border-top: 1px solid color-mix(in srgb, var(--bs-primary) 35%, transparent);
  box-shadow: 0 -4px 12px rgba(0, 0, 0, .06);
}

@media (max-width: 1000px) {
  .resizer {
    display: none;
  }

  .report {
    grid-template-columns: 1fr;
    grid-auto-rows: auto;
    overflow-y: auto;
  }

  .report .pane {
    overflow: visible;
    min-height: auto;
    border-right: none;
    border-bottom: 1px solid var(--bs-border-color);
  }

  .pane-list {
    max-height: 50vh;
    overflow-y: auto !important;
  }
}
</style>
