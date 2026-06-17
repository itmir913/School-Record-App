<script setup>
import { ref, watch } from 'vue'
import { useRecordStore } from '../stores/record.js'
import { X } from 'lucide-vue-next'
import DiffView from './DiffView.vue'

// 'content' = 버전별 내용 그대로, 'diff' = 직전 버전과의 차이
const viewMode = ref('content')

const props = defineProps({
  activityId: { type: Number, default: null },
  studentId:  { type: Number, default: null },
  activityName: { type: String, default: '' },
  studentName:  { type: String, default: '' },
})

const emit = defineEmits(['close'])

const recordStore = useRecordStore()

const LIMIT = 5

const entries = ref([])
const offset = ref(0)
const hasMore = ref(false)
const loading = ref(false)
const historyError = ref('')

const showNoteForm = ref(false)
const noteInput = ref('')
const saving = ref(false)
const saveError = ref('')

async function loadHistory(reset = false) {
  if (reset) {
    entries.value = []
    offset.value = 0
    historyError.value = ''
  }
  loading.value = true
  try {
    // LIMIT+1개를 조회해서 다음 페이지 존재 여부 확인
    const batch = await recordStore.fetchRecordHistory({
      activityId: props.activityId,
      studentId: props.studentId,
      limit: LIMIT + 1,
      offset: offset.value,
    })
    hasMore.value = batch.length > LIMIT
    const items = batch.slice(0, LIMIT)
    entries.value = [...entries.value, ...items]
    offset.value += items.length
  } catch (e) {
    historyError.value = String(e)
  } finally {
    loading.value = false
  }
}

watch(
  () => [props.activityId, props.studentId],
  ([aid, sid]) => {
    if (aid && sid) loadHistory(true)
  },
  { immediate: true }
)

// 각 항목의 "이전 내용"을 구한다 — 목록이 최신순이므로 index+1이 더 오래된 버전
function prevContent(index) {
  return entries.value[index + 1]?.content ?? ''
}

function formatDate(str) {
  const d = new Date(str.replace(' ', 'T') + 'Z')
  return d.toLocaleString('ko-KR', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit',
  })
}

async function saveManualSnapshot() {
  if (saving.value) return
  const note = noteInput.value.trim()
  if (!note) return
  saving.value = true
  saveError.value = ''
  try {
    await recordStore.saveHistorySnapshot({
      activityId: props.activityId,
      studentId: props.studentId,
      note,
    })
    noteInput.value = ''
    showNoteForm.value = false
    await loadHistory(true)
  } catch (e) {
    saveError.value = String(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="fixed inset-0 bg-overlay backdrop-blur-[6px] flex items-center justify-center z-[1000]" @click.self="emit('close')">
    <div class="bg-surface border border-line rounded-[14px] w-[680px] max-w-[95vw] h-[85vh] flex flex-col overflow-hidden">

      <!-- 헤더 -->
      <div class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-line shrink-0">
        <div class="flex items-center gap-1.5 text-base text-ink">
          <span class="font-semibold">{{ studentName }}</span>
          <span class="text-ink-5">/</span>
          <span class="text-blue-2">{{ activityName }}</span>
          <span class="text-ink-3 ml-1">히스토리</span>
        </div>
        <button
            class="bg-transparent border-none text-ink-5 cursor-pointer p-1 rounded-[6px] flex items-center hover:text-ink-2 hover:bg-line"
            @click="emit('close')"
        ><X :size="18"/></button>
      </div>

      <!-- 수동 저장 버튼 영역 -->
      <div class="px-6 py-3 border-b border-line shrink-0">
        <template v-if="!showNoteForm">
          <button
              class="py-[7px] px-4 rounded-lg border border-blue/40 bg-blue/[12%] text-blue-2 text-sm cursor-pointer hover:bg-blue/[22%]"
              @click="showNoteForm = true"
          >현재 버전 저장</button>
        </template>
        <template v-else>
          <div class="flex gap-2 items-center">
            <input
                v-model="noteInput"
                class="ui-input flex-1 w-auto py-[7px] px-3 rounded-lg text-sm border-line-2 placeholder:text-ink-5"
                placeholder="메모를 입력하세요 (필수)"
                maxlength="100"
                @keydown.enter="saveManualSnapshot"
                @keydown.esc="showNoteForm = false"
            />
            <button
                class="py-[7px] px-3.5 rounded-lg border-none bg-blue/70 text-ink text-sm cursor-pointer disabled:opacity-40 disabled:cursor-default enabled:hover:bg-blue/90"
                :disabled="!noteInput.trim() || saving"
                @click="saveManualSnapshot"
            >
              {{ saving ? '저장 중...' : '저장' }}
            </button>
            <button
                class="py-[7px] px-3 rounded-lg border border-line bg-transparent text-ink-2 text-sm cursor-pointer hover:bg-line"
                @click="showNoteForm = false; noteInput = ''"
            >취소</button>
          </div>
          <p v-if="saveError" class="text-sm text-red bg-red/[8%] border border-red/20 rounded-lg px-3 py-2 mt-1 m-0">{{ saveError }}</p>
        </template>
      </div>

      <!-- 뷰 모드 토글 -->
      <div class="flex gap-1.5 px-6 py-2.5 border-b border-line shrink-0">
        <button
            class="py-[5px] px-3.5 rounded-[7px] border text-xs cursor-pointer transition-colors"
            :class="viewMode === 'content'
              ? 'bg-blue/[18%] border-blue/50 text-blue-2'
              : 'border-line bg-transparent text-ink-5 hover:bg-line hover:text-ink-3'"
            @click="viewMode = 'content'"
        >원문 보기</button>
        <button
            class="py-[5px] px-3.5 rounded-[7px] border text-xs cursor-pointer transition-colors"
            :class="viewMode === 'diff'
              ? 'bg-blue/[18%] border-blue/50 text-blue-2'
              : 'border-line bg-transparent text-ink-5 hover:bg-line hover:text-ink-3'"
            @click="viewMode = 'diff'"
        >수정된 부분(diff) 보기</button>
      </div>

      <!-- 히스토리 목록 -->
      <div class="overflow-y-auto px-6 py-4 flex-1 min-h-0 flex flex-col gap-3">
        <p v-if="historyError" class="text-sm text-red bg-red/[8%] border border-red/20 rounded-lg px-3 py-2 m-0">{{ historyError }}</p>

        <div v-if="entries.length === 0 && !loading && !historyError"
             class="text-sm text-ink-5 text-center py-6">
          저장된 히스토리가 없습니다.
        </div>

        <div v-for="(entry, idx) in entries" :key="entry.id"
             class="border border-line rounded-btn overflow-hidden shrink-0">
          <div class="flex items-center gap-2 px-3.5 py-2 bg-base border-b border-line">
            <span class="text-blue text-[10px]">●</span>
            <span class="text-xs text-blue-2">{{ formatDate(entry.changed_at) }}</span>
            <span v-if="entry.note" class="text-xs text-amber">📌 {{ entry.note }}</span>
            <span v-else class="text-[11px] text-ink-5 bg-line px-[7px] py-px rounded-[4px]">자동</span>
          </div>
          <div class="px-3.5 py-2.5 bg-base">
            <DiffView v-if="viewMode === 'diff'" :before="prevContent(idx)" :after="entry.content" />
            <span v-else class="text-sm leading-relaxed whitespace-pre-wrap break-all text-ink-2">{{ entry.content }}</span>
          </div>
        </div>

        <div v-if="loading" class="text-sm text-ink-5 text-center py-6">불러오는 중...</div>

        <button
            v-if="hasMore && !loading"
            class="self-center py-[7px] px-5 rounded-lg border border-line bg-transparent text-blue-2 text-sm cursor-pointer mt-1 hover:bg-line"
            @click="loadHistory(false)"
        >{{ LIMIT }}개 더 불러오기</button>
      </div>

    </div>
  </div>
</template>
