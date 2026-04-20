<script setup>
import {computed, ref, watch} from 'vue'
import {Trash2, X} from 'lucide-vue-next'

const props = defineProps({
  mode: {type: String, default: 'add'}, // 'add' | 'edit'
  area: {type: Object, default: null},
  allActivities: {type: Array, default: () => []},
})

const emit = defineEmits(['close', 'saved', 'deleted'])

const name = ref('')
const byteLimit = ref(1500)
const error = ref('')
const confirmDelete = ref(false)
const selectedIds = ref(new Set())
const sortedActivities = computed(() =>
    [...props.allActivities].sort((a, b) => a.name.localeCompare(b.name, 'ko'))
)

// 편집 모드 진입 시 기존 값 채우기
watch(
    () => props.area,
    (a) => {
      if (a) {
        name.value = a.name
        byteLimit.value = a.byte_limit
        selectedIds.value = new Set(a.activities.map(x => x.id))
      } else {
        name.value = ''
        byteLimit.value = 1500
        selectedIds.value = new Set()
      }
      error.value = ''
      confirmDelete.value = false
    },
    {immediate: true}
)

function validate() {
  if (!name.value.trim()) {
    error.value = '영역 이름을 입력해주세요.'
    return false
  }
  if (!byteLimit.value || byteLimit.value <= 0) {
    error.value = '바이트 수 제한은 1 이상이어야 합니다.'
    return false
  }
  return true
}

function toggleActivity(id) {
  const next = new Set(selectedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedIds.value = next
}

function submit() {
  if (!validate()) return
  emit('saved', {
    name: name.value.trim(),
    byteLimit: Number(byteLimit.value),
    activityIds: [...selectedIds.value],
  })
}

function handleDelete() {
  if (!confirmDelete.value) {
    confirmDelete.value = true
    return
  }
  emit('deleted')
}
</script>

<template>
  <div class="overlay">
    <div class="modal">

      <!-- 헤더 -->
      <div class="modal-header">
        <h2 class="modal-title">{{ mode === 'add' ? '영역 추가' : '영역 수정' }}</h2>
        <button class="close-btn" @click="emit('close')">
          <X :size="18"/>
        </button>
      </div>

      <!-- 2단 바디 -->
      <div class="modal-body">

        <!-- 좌측: 기본 정보 -->
        <div class="pane pane-left">
          <p class="pane-title">기본 정보</p>

          <div class="field">
            <label class="field-label">영역 이름 <span class="required">*</span></label>
            <input
                v-model="name"
                class="field-input"
                placeholder="예: 자율활동"
                @keydown.enter="submit"
            />
          </div>

          <div class="field">
            <label class="field-label">바이트 수 제한 <span class="required">*</span></label>
            <div class="input-row">
              <input
                  v-model.number="byteLimit"
                  type="number"
                  min="1"
                  class="field-input"
                  placeholder="1500"
                  @keydown.enter="submit"
              />
              <span class="input-unit">Bytes</span>
            </div>
            <p class="field-hint">나이스 기준 최대 입력 가능한 바이트 수</p>
          </div>

          <!-- 에러 -->
          <p v-if="error" class="form-error">{{ error }}</p>
        </div>

        <!-- 구분선 -->
        <div class="pane-divider"/>

        <!-- 우측: 활동 선택 -->
        <div class="pane pane-right">
          <div class="pane-title-row">
            <p class="pane-title">포함할 활동</p>
            <span v-if="allActivities.length > 0" class="selected-count">
              {{ selectedIds.size }}개 선택됨
            </span>
          </div>

          <p v-if="allActivities.length === 0" class="empty-hint">
            등록된 활동이 없습니다.<br>활동 관리에서 먼저 추가하세요.
          </p>
          <div v-else class="chip-scroll">
            <button
                v-for="act in sortedActivities"
                :key="act.id"
                type="button"
                class="act-chip"
                :class="{'act-chip--on': selectedIds.has(act.id)}"
                @click="toggleActivity(act.id)"
            >{{ act.name }}
            </button>
          </div>
        </div>
      </div>

      <!-- 푸터 -->
      <div class="modal-footer">
        <div class="footer-left">
          <template v-if="mode === 'edit'">
            <button
                v-if="!confirmDelete"
                class="btn-delete"
                @click="handleDelete"
            >
              <Trash2 :size="15"/>
              삭제
            </button>
            <div v-else class="confirm-row">
              <span class="confirm-text">정말 삭제할까요?</span>
              <button class="btn-cancel-sm" @click="confirmDelete = false">취소</button>
              <button class="btn-delete-confirm" @click="handleDelete">삭제</button>
            </div>
          </template>
        </div>

        <div class="footer-right">
          <button class="btn-cancel" @click="emit('close')">취소</button>
          <button class="btn-submit" @click="submit">
            {{ mode === 'add' ? '추가' : '저장' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(4, 6, 12, 0.75);
  backdrop-filter: blur(6px);
}

.modal {
  width: 100%;
  max-width: 920px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7);
  overflow: hidden;
}

/* 헤더 */
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 0;
}

.modal-title {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: none;
  border: none;
  color: #3d5580;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.close-btn:hover {
  background-color: #1a2035;
  color: #7ba3d4;
}

/* 2단 바디 */
.modal-body {
  display: flex;
  align-items: stretch;
  padding: 20px 0 4px;
  min-height: 380px;
}

.pane {
  display: flex;
  flex-direction: column;
  gap: 18px;
  flex: 1;
  padding: 0 24px 16px;
}

.pane-divider {
  width: 1px;
  background-color: #1a2035;
  flex-shrink: 0;
  margin: 4px 0 20px;
}

.pane-title {
  font-size: 15px;
  font-weight: 600;
  color: #7ba3d4;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  margin: 0;
}

.pane-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.selected-count {
  font-size: 15px;
  color: #7ba3d4;
}

/* 필드 */
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 15px;
  font-weight: 600;
  color: #93afd4;
}

.required {
  color: #f87171;
}

.field-input {
  width: 100%;
  padding: 10px 14px;
  font-size: 16px;
  background-color: #080b14;
  border: 1px solid #1a2035;
  border-radius: 10px;
  color: #e2e8f0;
  outline: none;
  transition: border-color 0.15s;
  box-sizing: border-box;
}

.field-input:focus {
  border-color: rgba(59, 91, 219, 0.6);
}

.field-input::placeholder {
  color: #2a3a50;
}

.input-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.input-row .field-input {
  flex: 1;
}

.input-unit {
  font-size: 16px;
  color: #7ba3d4;
  white-space: nowrap;
}

.field-hint {
  font-size: 15px;
  color: #7ba3d4;
  margin: 0;
  text-align: right;
}

.form-error {
  font-size: 15px;
  color: #f87171;
  background-color: rgba(248, 113, 113, 0.08);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: 8px;
  padding: 10px 14px;
  margin: 0;
}

/* 우측 패널 */
.empty-hint {
  font-size: 15px;
  color: #7ba3d4;
  line-height: 1.7;
  margin: 0;
}

.pane-right {
  display: flex;
  flex-direction: column;
}

.chip-scroll {
  display: flex;
  flex-wrap: wrap;
  align-content: flex-start;
  gap: 8px;
  flex: 1;
  overflow-y: auto;
  overflow-y: auto;
  padding-right: 4px;
}

.chip-scroll::-webkit-scrollbar {
  width: 4px;
}

.chip-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.chip-scroll::-webkit-scrollbar-thumb {
  background-color: #1a2035;
  border-radius: 4px;
}

.act-chip {
  padding: 7px 16px;
  border-radius: 20px;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #1a2035;
  background-color: #0b1020;
  color: #7ba3d4;
  transition: border-color 0.15s, background-color 0.15s, color 0.15s;
  white-space: nowrap;
}

.act-chip:hover {
  border-color: #2a3a58;
  color: #93c5fd;
}

.act-chip--on {
  border-color: rgba(59, 91, 219, 0.45);
  background-color: rgba(59, 91, 219, 0.15);
  color: #7ba8f0;
}

.act-chip--on:hover {
  background-color: rgba(59, 91, 219, 0.22);
}

/* 푸터 */
.modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px 20px;
  border-top: 1px solid #1a2035;
  gap: 12px;
}

.footer-left {
  display: flex;
  align-items: center;
}

.footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-delete {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 10px;
  background-color: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.2);
  color: #f87171;
  cursor: pointer;
  font-size: 15px;
  font-weight: 500;
  transition: background-color 0.15s;
}

.btn-delete:hover {
  background-color: rgba(239, 68, 68, 0.18);
}

.confirm-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.confirm-text {
  font-size: 15px;
  color: #f87171;
}

.btn-cancel-sm {
  padding: 6px 12px;
  border-radius: 8px;
  background-color: #1a2035;
  border: none;
  color: #93afd4;
  cursor: pointer;
  font-size: 15px;
  transition: background-color 0.15s;
}

.btn-cancel-sm:hover {
  background-color: #222e48;
}

.btn-delete-confirm {
  padding: 6px 12px;
  border-radius: 8px;
  background-color: rgba(239, 68, 68, 0.8);
  border: none;
  color: white;
  cursor: pointer;
  font-size: 15px;
  font-weight: 600;
  transition: background-color 0.15s;
}

.btn-delete-confirm:hover {
  background-color: #ef4444;
}

.btn-cancel {
  padding: 9px 18px;
  border-radius: 10px;
  background-color: #131c30;
  border: 1px solid #1a2035;
  color: #7ba3d4;
  cursor: pointer;
  font-size: 16px;
  transition: background-color 0.15s;
}

.btn-cancel:hover {
  background-color: #1a2640;
}

.btn-submit {
  padding: 9px 22px;
  border-radius: 10px;
  background-color: #3b5bdb;
  border: none;
  color: white;
  cursor: pointer;
  font-size: 16px;
  font-weight: 600;
  transition: background-color 0.15s;
}

.btn-submit:hover {
  background-color: #4c6ef5;
}
</style>
