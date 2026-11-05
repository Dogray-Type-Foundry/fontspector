<script setup lang="ts">
import { postToWorker } from '../workersingleton';
import { state } from '../store';
import { PROFILES } from '../constants';
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import Dropzone from 'dropzone';
import { Profile, RunCheckRequest } from '../types';

function runChecks() {
  const files: Record<string, Uint8Array> = {};
  for (const filename in state.fonts) {
    files[filename] = state.fonts[filename].file;
  }
  let request: RunCheckRequest =
  {
    id: "run_checks",
    profile: state.selectedProfile,
    files,
    loglevels: state.logLevel,
    fulllists: state.fullLists
  };
  postToWorker(request);
}

const profileCols = computed(() => {
  const keys = Object.keys(PROFILES);
  const mid = Math.ceil(keys.length / 2);
  const col0 = keys.slice(0, mid).map(k => ({ id: k, label: PROFILES[k as Profile] }));
  const col1 = keys.slice(mid).map(k => ({ id: k, label: PROFILES[k as Profile] }));
  return [col0, col1];
});


const dropzoneElement = ref<HTMLElement | null>(null);

onMounted(() => {
  initDropzone();
  postToWorker({ id: "justload" });
});

watch(() => state.view, (newView) => {
  if (newView === 'start') {
    nextTick(() => initDropzone());
  }
});


function initDropzone() {
  if (!dropzoneElement.value) return;
  const dz = new Dropzone(dropzoneElement.value, {
    url: "https://127.0.0.1/", // ignored
    maxFilesize: 10,
    autoProcessQueue: false,
  });
  dz.on("addedfile", (file: any) => {
    const reader = new FileReader();
    reader.addEventListener("loadend", (event: any) => {
      const filedata = new Uint8Array(event.target.result);
      // harfbuzzjs integration
      const currentHbjs = (window as any).hbjs;
      if (currentHbjs) {
        const blob = currentHbjs.createBlob(filedata);
        const face = currentHbjs.createFace(blob, 0);
        const font = currentHbjs.createFont(face);
        state.fonts[file.name] = {
          name: file.name,
          file: filedata,
          blob,
          face,
          font,
        };
      } else {
        state.fonts[file.name] = {
          name: file.name,
          file: filedata,
          blob: null, face: null, font: null
        };
      }
    });
    reader.readAsArrayBuffer(file);
  });
}

</script>
<template>
  <div class="modal-dialog bg-gradient modal-fullscreen">
    <div class="modal-content">
      <div class="modal-body">
        <div class="container">
          <div class="row justify-content-between">
            <div class="align-self-center mx-auto rounded pt-0 pl-3 pr-3 bg-modal shadow-lg">
              <h1 class="start-title">Dogray <span>Font Inspector</span></h1>
              <div id="dropzone-container">
                <div class="dropzone" ref="dropzoneElement">
                  <div class="dz-message">Drop your fonts here.</div>
                </div>
              </div>
              <div class="check-profile p-3 m-3">
                <div class="row">
                  <div class="col">
                    <h6>Log levels</h6>
                    <select class="form-select form-select-sm w-auto" v-model="state.logLevel">
                      <option value="PASS">PASS</option>
                      <option value="INFO">INFO</option>
                      <option value="WARN">WARN</option>
                      <option value="FAIL">FAIL</option>
                    </select>
                  </div>
                  <div class="col">
                    <div class="form-check">
                      <input id="full-lists" class="form-check-input" type="checkbox" v-model="state.fullLists" />
                      <label class="form-check-label" for="full-lists">Display full lists?</label>
                    </div>
                  </div>
                </div>
              </div>
              <div class="check-profile p-3 m-3">
                <h6>Check profile</h6>
                <div class="row">
                  <div class="col" v-for="(profileCol, colIndex) in profileCols" :key="colIndex">
                    <div class="form-check" v-for="profile in profileCol" :key="profile.id">
                      <input class="form-check-input" type="radio" name="profileGroup" :id="'profile-' + profile.id"
                        :value="profile.id" v-model="state.selectedProfile" />
                      <label class="form-check-label" :for="'profile-' + profile.id">
                        {{ profile.label }}
                      </label>
                    </div>
                  </div>
                </div>
                <div class="mx-auto" style="width: 200px">
                  <button v-if="!state.loading" class="btn mt-3 btn-primary w-100" @click="runChecks">
                    Inspect!
                  </button>
                  <button v-if="state.loading" class="btn mt-3 btn-primary w-100" disabled>
                    <span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span>
                    Loading
                  </button>
                </div>
                <div class="mx-auto mt-2 mb-0 text-muted text-center">
                  <p>
                    <small>No files are uploaded; your fonts stay in your browser.</small>
                  </p>
                </div>
                <div v-if="!state.loading" class="mx-auto mt-2 mb-0 text-center">
                  <p>
                    <button class="btn btn-outline-primary" @click="state.view = 'listChecks'">
                      List all checks
                    </button>
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
.bg-modal {
  background-color: var(--bs-tertiary-bg);
}

.start-title {
  text-align: center;
  font-weight: 700;
  font-size: clamp(2rem, 5vw, 3.25rem);
  letter-spacing: -.02em;
  color: var(--bs-primary);
  margin: 2.5rem 0 1.5rem;
}

.start-title span {
  font-weight: 400;
  color: var(--dogray-accent);
}

.dropzone {
  border: 2px dashed color-mix(in srgb, var(--dogray-accent) 60%, transparent);
  border-radius: 70px;
  background-color: color-mix(in srgb, var(--dogray-accent) 8%, var(--bs-body-bg));
  color: var(--bs-body-color);
  min-height: 9rem;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  margin: 0 1rem;
}

[data-bs-theme=dark] .start-title {
  color: var(--bs-primary-text-emphasis);
}
</style>
