<script setup>
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { X } from 'lucide-vue-next'
import DiffView from './DiffView.vue'

const props = defineProps({
  activityId: { type: Number, default: null },
  studentId:  { type: Number, default: null },
  activityName: { type: String, default: '' },
  studentName:  { type: String, default: '' },
})

const emit = defineEmits(['close'])

const LIMIT = 5

const entries = ref([])
const offset = ref(0)
const hasMore = ref(false)
const loading = ref(false)

const showNoteForm = ref(false)
const noteInput = ref('')
const saving = ref(false)

async function loadHistory(reset = false) {
  if (reset) {
    entries.value = []
    offset.value = 0
  }
  loading.value = true
  try {
    // LIMIT+1개를 조회해서 다음 페이지 존재 여부 확인
    const batch = await invoke('get_record_history', {
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
    console.error(e)
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
  const note = noteInput.value.trim()
  if (!note) return
  saving.value = true
  try {
    await invoke('save_history_snapshot', {
      activityId: props.activityId,
      studentId: props.studentId,
      note,
    })
    noteInput.value = ''
    showNoteForm.value = false
    await loadHistory(true)
  } catch (e) {
    console.error(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal">

      <!-- 헤더 -->
      <div class="modal-header">
        <div class="modal-title">
          <span class="modal-student">{{ studentName }}</span>
          <span class="modal-sep">/</span>
          <span class="modal-activity">{{ activityName }}</span>
          <span class="modal-label">히스토리</span>
        </div>
        <button class="btn-close" @click="emit('close')"><X :size="18"/></button>
      </div>

      <!-- 수동 저장 버튼 영역 -->
      <div class="manual-save-area">
        <template v-if="!showNoteForm">
          <button class="btn-manual" @click="showNoteForm = true">현재 버전 저장</button>
        </template>
        <template v-else>
          <div class="note-form">
            <input
              v-model="noteInput"
              class="note-input"
              placeholder="메모를 입력하세요 (필수)"
              maxlength="100"
              @keydown.enter="saveManualSnapshot"
              @keydown.esc="showNoteForm = false"
            />
            <button class="btn-confirm" :disabled="!noteInput.trim() || saving" @click="saveManualSnapshot">
              {{ saving ? '저장 중...' : '저장' }}
            </button>
            <button class="btn-cancel" @click="showNoteForm = false; noteInput = ''">취소</button>
          </div>
        </template>
      </div>

      <!-- 히스토리 목록 -->
      <div class="history-list">
        <div v-if="entries.length === 0 && !loading" class="empty-msg">
          저장된 히스토리가 없습니다.
        </div>

        <div v-for="(entry, idx) in entries" :key="entry.id" class="history-item">
          <div class="item-meta">
            <span class="item-dot">●</span>
            <span class="item-date">{{ formatDate(entry.changed_at) }}</span>
            <span v-if="entry.note" class="item-note">📌 {{ entry.note }}</span>
            <span v-else class="item-auto">자동</span>
          </div>
          <div class="item-diff">
            <DiffView :before="prevContent(idx)" :after="entry.content" />
          </div>
        </div>

        <div v-if="loading" class="loading-msg">불러오는 중...</div>

        <button
          v-if="hasMore && !loading"
          class="btn-more"
          @click="loadHistory(false)"
        >{{ LIMIT }}개 더 불러오기</button>
      </div>

    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: #0d1220;
  border: 1px solid #1a2035;
  border-radius: 14px;
  width: 680px;
  max-width: 95vw;
  height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 16px;
  border-bottom: 1px solid #1a2035;
  flex-shrink: 0;
}

.modal-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 15px;
  color: #e2e8f0;
}

.modal-student { font-weight: 600; }
.modal-sep { color: #4a5f8a; }
.modal-activity { color: #8aaaf8; }
.modal-label { color: #a0bcd8; margin-left: 4px; }

.btn-close {
  background: none;
  border: none;
  color: #6080a0;
  cursor: pointer;
  padding: 4px;
  border-radius: 6px;
  display: flex;
  align-items: center;
}
.btn-close:hover { color: #c8ddf0; background: #1a2035; }

/* 수동 저장 영역 */
.manual-save-area {
  padding: 12px 24px;
  border-bottom: 1px solid #1a2035;
  flex-shrink: 0;
}

.btn-manual {
  padding: 7px 16px;
  border-radius: 8px;
  border: 1px solid rgba(59, 91, 219, 0.4);
  background: rgba(59, 91, 219, 0.12);
  color: #a8c8ff;
  font-size: 13px;
  cursor: pointer;
}
.btn-manual:hover { background: rgba(59, 91, 219, 0.22); }

.note-form {
  display: flex;
  gap: 8px;
  align-items: center;
}

.note-input {
  flex: 1;
  padding: 7px 12px;
  border-radius: 8px;
  border: 1px solid #2a3a60;
  background: #080b14;
  color: #e2e8f0;
  font-size: 13px;
  outline: none;
}
.note-input:focus { border-color: rgba(59, 91, 219, 0.6); }

.btn-confirm {
  padding: 7px 14px;
  border-radius: 8px;
  border: none;
  background: rgba(59, 91, 219, 0.7);
  color: #e2e8f0;
  font-size: 13px;
  cursor: pointer;
}
.btn-confirm:disabled { opacity: 0.4; cursor: default; }
.btn-confirm:not(:disabled):hover { background: rgba(59, 91, 219, 0.9); }

.btn-cancel {
  padding: 7px 12px;
  border-radius: 8px;
  border: 1px solid #1a2035;
  background: none;
  color: #a0bcd8;
  font-size: 13px;
  cursor: pointer;
}
.btn-cancel:hover { background: #1a2035; }

/* 히스토리 목록 */
.history-list {
  overflow-y: auto;
  padding: 16px 24px;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.empty-msg, .loading-msg {
  font-size: 14px;
  color: #6080a0;
  text-align: center;
  padding: 24px 0;
}

.history-item {
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
  flex-shrink: 0;
}

.item-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  background: #080b14;
  border-bottom: 1px solid #1a2035;
}

.item-dot { color: #3b5bdb; font-size: 10px; }
.item-date { font-size: 12px; color: #8aaaf8; }
.item-note { font-size: 12px; color: #f0c060; }
.item-auto { font-size: 11px; color: #4a5f8a; background: #1a2035; padding: 1px 7px; border-radius: 4px; }

.item-diff {
  padding: 10px 14px;
  background: #0a0e1a;
}

.btn-more {
  align-self: center;
  padding: 7px 20px;
  border-radius: 8px;
  border: 1px solid #1a2035;
  background: none;
  color: #8aaaf8;
  font-size: 13px;
  cursor: pointer;
  margin-top: 4px;
}
.btn-more:hover { background: #1a2035; }
</style>
