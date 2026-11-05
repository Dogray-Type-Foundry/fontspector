<script setup lang="ts">
import { EMOJIS, PROFILES, STATUS_LABELS } from '../constants';
import { state, resetState } from '../store';
import { resetFilters, selection } from '../results';
import { useColorMode } from 'bootstrap-vue-next'

const mode = useColorMode();
function startOver() {
    resetState();
    resetFilters();
    selection.checkId = null;
}
function toggleView() {
    state.view = state.view === 'classic' ? 'problem' : 'classic';
}

const changeColor = () => {
    mode.value = mode.value === 'dark' ? 'light' : 'dark'
}


function downloadReport() {
    const html = document.documentElement.outerHTML;
    const blob = new Blob([html], { type: 'text/html' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'dogray-font-inspector-report.html';
    a.click();
}

</script>

<template>
    <nav class="topbar bg-navbar">
        <a class="back" href="#" @click.prevent="startOver" title="Check other fonts">←</a>
        <div class="wordmark">Dogray <span>Font Inspector</span></div>
        <div class="title">
            <div class="font-name" id="font-name">{{ state.currentFontName }}</div>
            <div class="sub">
                {{ PROFILES[state.selectedProfile] }} · engine v{{ state.version }}
            </div>
        </div>

        <div v-if="state.view === 'classic'" id="badges" class="ms-3">
            <span v-for="(label, status) in STATUS_LABELS" :key="status" class="mx-2" :title="label">
                {{ EMOJIS[status] }} <span>{{ state.counts[status] }}</span>
            </span>
        </div>

        <div class="actions">
            <div class="form-check form-switch m-0">
                <input class="form-check-input" type="checkbox" role="switch" id="view-switch"
                    :checked="state.view === 'classic'" @change="toggleView" />
                <label class="form-check-label small" for="view-switch">Classic view</label>
            </div>
            <button class="btn btn-sm btn-outline-secondary" @click="downloadReport">Download report</button>
            <button class="btn btn-sm btn-link text-decoration-none" @click="changeColor"
                :title="mode === 'dark' ? 'Light mode' : 'Dark mode'">{{ mode === 'dark' ? '☀️' : '🌙' }}</button>
        </div>
    </nav>
</template>

<style>
.bg-navbar {
    background-color: var(--bs-body-bg);
    border-bottom: 1px solid var(--bs-border-color);
}

.wordmark {
    font-weight: 700;
    font-size: 1.05rem;
    letter-spacing: -.01em;
    color: var(--bs-primary);
    padding-right: .9rem;
    margin-right: .15rem;
    border-right: 1px solid var(--bs-border-color);
    white-space: nowrap;
}

[data-bs-theme=dark] .wordmark {
    color: var(--bs-primary-text-emphasis);
}

.wordmark span {
    font-weight: 400;
    color: var(--dogray-accent);
}

.topbar {
    display: flex;
    align-items: center;
    gap: .75rem;
    padding: .5rem 1rem;
    flex: none;
}

.topbar .back {
    font-size: 1.4rem;
    line-height: 1;
    color: var(--bs-secondary-color);
    text-decoration: none;
}

.topbar .title {
    min-width: 0;
}

.topbar .font-name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.topbar .sub {
    font-size: .75rem;
    color: var(--bs-secondary-color);
}

.topbar .actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: .75rem;
}
</style>
