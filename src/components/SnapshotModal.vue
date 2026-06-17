<script setup>
import {onMounted, ref} from 'vue'
import {useSnapshotStore} from '../stores/snapshot.js'
import {GitBranch, Plus, RotateCcw, X} from 'lucide-vue-next'

const emit = defineEmits(['close', 'restored'])

const snapshotStore = useSnapshotStore()

const snapshots = ref([])
const loading = ref(false)
const loadError = ref('')
const creating = ref(false)
const createError = ref('')
const restoring = ref(false)
const restoreError = ref('')
const memoInput = ref('')
const showCreateForm = ref(false)
const confirmRestoreId = ref(null)

async function loadSnapshots() {
  loading.value = true
  loadError.value = ''
  try {
    snapshots.value = await snapshotStore.fetchSnapshots()
  } catch (e) {
    loadError.value = String(e)
  } finally {
    loading.value = false
  }
}

async function handleCreate() {
  if (creating.value) return
  creating.value = true
  createError.value = ''
  try {
    const newSnap = await snapshotStore.createSnapshot(memoInput.value.trim())
    snapshots.value.unshift(newSnap)
    memoInput.value = ''
    showCreateForm.value = false
  } catch (e) {
    createError.value = String(e)
  } finally {
    creating.value = false
  }
}

async function handleRestore() {
  if (confirmRestoreId.value === null) return
  restoring.value = true
  restoreError.value = ''
  try {
    await snapshotStore.restoreSnapshot(confirmRestoreId.value)
    confirmRestoreId.value = null
    emit('restored')
    emit('close')
  } catch (e) {
    restoreError.value = String(e)
  } finally {
    restoring.value = false
  }
}

function formatDate(str) {
  const d = new Date(str.replace(' ', 'T') + 'Z')
  return d.toLocaleString('ko-KR', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit',
  })
}

onMounted(loadSnapshots)
</script>

<template>
  <div class="fixed inset-0 bg-overlay backdrop-blur-[6px] flex items-center justify-center z-[1000]">
    <div class="bg-surface border border-line rounded-[14px] w-[780px] h-[560px] max-w-[95vw] max-h-[80vh] flex flex-col overflow-hidden">

      <!-- 헤더 -->
      <div class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-line shrink-0">
        <div class="flex items-center gap-2 text-base font-semibold text-ink">
          <GitBranch :size="16" class="text-blue-2"/>
          <span>스냅샷(Snapshot)</span>
        </div>
        <button
            class="bg-transparent border-none text-ink-5 cursor-pointer p-1 rounded-[6px] flex items-center hover:text-ink-2 hover:bg-line"
            @click="emit('close')"
        ><X :size="18"/></button>
      </div>

      <!-- 새 스냅샷 생성 영역 -->
      <div class="px-6 py-3.5 border-b border-line shrink-0">
        <template v-if="!showCreateForm">
          <button
              class="flex items-center gap-1.5 py-[7px] px-4 rounded-lg border border-blue/40 bg-blue/[12%] text-blue-2 text-sm cursor-pointer hover:bg-blue/[22%]"
              @click="showCreateForm = true"
          >
            <Plus :size="14"/>
            현재 상태로 스냅샷 생성
          </button>
        </template>
        <template v-else>
          <div class="flex gap-2 items-center">
            <input
                v-model="memoInput"
                class="ui-input flex-1 w-auto py-[7px] px-3 rounded-lg text-sm border-line-2 placeholder:text-ink-5"
                placeholder="메모 (선택)"
                maxlength="100"
                autofocus
                @keydown.enter="handleCreate"
                @keydown.esc="showCreateForm = false; memoInput = ''"
            />
            <button
                class="py-[7px] px-3.5 rounded-lg border-none bg-blue/70 text-ink text-sm cursor-pointer whitespace-nowrap disabled:opacity-40 disabled:cursor-default enabled:hover:bg-blue/90"
                :disabled="creating"
                @click="handleCreate"
            >
              {{ creating ? '생성 중...' : '생성' }}
            </button>
            <button
                class="py-[7px] px-3 rounded-lg border border-line bg-transparent text-ink-2 text-sm cursor-pointer whitespace-nowrap hover:bg-line"
                @click="showCreateForm = false; memoInput = ''"
            >
              취소
            </button>
          </div>
          <p v-if="createError" class="text-xs text-red m-0 mt-1">{{ createError }}</p>
        </template>
      </div>

      <!-- 스냅샷 목록 -->
      <div class="overflow-y-auto flex-1 min-h-0 px-4 py-3 flex flex-col gap-2">
        <div v-if="loading" class="text-sm text-ink-5 text-center py-8">불러오는 중...</div>
        <div v-else-if="loadError" class="text-sm text-red text-center py-8">{{ loadError }}</div>
        <div v-else-if="snapshots.length === 0" class="text-sm text-ink-5 text-center py-8">
          저장된 스냅샷이 없습니다.
        </div>

        <div v-for="snap in snapshots" :key="snap.id"
             class="border border-line rounded-btn overflow-hidden shrink-0">
          <div class="flex items-center gap-2 px-3.5 py-2.5 bg-base border-b border-line">
            <span class="text-blue text-[10px]">●</span>
            <span class="text-sm text-blue-2">{{ formatDate(snap.created_at) }}</span>
            <span v-if="snap.memo"
                  class="text-xs text-amber overflow-hidden text-ellipsis whitespace-nowrap">{{ snap.memo }}</span>
          </div>

          <div class="flex items-center gap-2 px-3.5 py-2">
            <template v-if="confirmRestoreId !== snap.id">
              <button
                  class="flex items-center gap-[5px] py-[5px] px-3 rounded-[7px] border border-line-2 bg-transparent text-blue-2 text-xs cursor-pointer hover:bg-line"
                  @click="confirmRestoreId = snap.id"
              >
                <RotateCcw :size="13"/>
                복원
              </button>
            </template>
            <template v-else>
              <span class="text-xs text-amber flex-1">이 시점으로 복원합니다. <strong><u>저장되지 않은 모든 내용이 삭제</u></strong>되고 과거 스냅샷 시점으로 덮어써집니다.</span>
              <button
                  class="py-[5px] px-3 rounded-[7px] border-none bg-red/60 text-red/80 text-xs cursor-pointer whitespace-nowrap disabled:opacity-40 disabled:cursor-default enabled:hover:bg-red/85"
                  :disabled="restoring"
                  @click="handleRestore"
              >
                {{ restoring ? '복원 중...' : '확인' }}
              </button>
              <button
                  class="py-[5px] px-3 rounded-[7px] border border-line bg-transparent text-ink-2 text-xs cursor-pointer whitespace-nowrap hover:bg-line"
                  @click="confirmRestoreId = null; restoreError = ''"
              >취소</button>
              <p v-if="restoreError" class="text-xs text-red m-0">{{ restoreError }}</p>
            </template>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>
