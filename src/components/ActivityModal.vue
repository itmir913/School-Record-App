<script setup>
import {computed, ref, watch} from 'vue'
import {AlertTriangle, Info, Trash2, X} from 'lucide-vue-next'

const props = defineProps({
  mode: {type: String, default: 'add'}, // 'add' | 'edit'
  activity: {type: Object, default: null},
  allAreas: {type: Array, default: () => []},
})

const emit = defineEmits(['close', 'saved', 'deleted'])

const name = ref('')
const error = ref('')
const confirmDelete = ref(false)
const selectedAreaIds = ref(new Set())

watch(
    () => props.activity,
    (a) => {
      name.value = a ? a.name : ''
      selectedAreaIds.value = new Set(a ? a.areas.map(x => x.id) : [])
      error.value = ''
      confirmDelete.value = false
    },
    {immediate: true}
)

const multiAreaWarning = computed(() => selectedAreaIds.value.size >= 2)
const sortedAreas = computed(() =>
    [...props.allAreas].sort((a, b) => a.name.localeCompare(b.name, 'ko'))
)

function toggleArea(id) {
  const next = new Set(selectedAreaIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedAreaIds.value = next
}

function validate() {
  if (!name.value.trim()) {
    error.value = '활동 이름을 입력해주세요.'
    return false
  }
  return true
}

function submit() {
  if (!validate()) return
  emit('saved', {
    name: name.value.trim(),
    areaIds: [...selectedAreaIds.value],
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
        <h2 class="modal-title">{{ mode === 'add' ? '활동 추가' : '활동 수정' }}</h2>
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
            <label class="field-label">활동 이름 <span class="required">*</span></label>
            <input
                v-model="name"
                class="field-input"
                placeholder="예: 학급자치활동"
                @keydown.enter="submit"
            />
            <p class="field-hint">
              영역(Area) 안에 포함될 세부 활동명입니다.
            </p>
          </div>

          <!-- 삭제 경고 (편집 + 확인 단계) -->
          <div v-if="mode === 'edit' && confirmDelete" class="delete-warning">
            <div class="warning-header">
              <AlertTriangle :size="16" class="warning-icon"/>
              <span class="warning-title">정말 삭제하시겠습니까?</span>
            </div>
            <p class="warning-body">
              이 활동을 삭제하면 이 활동에 속한 <strong>학생의 생기부 문장도 함께 삭제</strong>되며 복구할 수 없습니다.
            </p>
          </div>

          <!-- 에러 -->
          <p v-if="error" class="form-error">{{ error }}</p>
        </div>

        <!-- 구분선 -->
        <div class="pane-divider"/>

        <!-- 우측: 영역 선택 -->
        <div class="pane pane-right">
          <div class="pane-title-row">
            <p class="pane-title">포함할 영역</p>
            <span v-if="allAreas.length > 0" class="selected-count">
              {{ selectedAreaIds.size }}개 선택
            </span>
          </div>

          <p v-if="allAreas.length === 0" class="empty-hint">
            등록된 영역이 없습니다.<br>영역 관리에서 먼저 추가하세요.
          </p>
          <div v-else class="chip-scroll">
            <button
                v-for="area in sortedAreas"
                :key="area.id"
                type="button"
                class="area-chip"
                :class="{'area-chip--on': selectedAreaIds.has(area.id)}"
                @click="toggleArea(area.id)"
            >{{ area.name }}
            </button>
          </div>

          <!-- 복수 영역 선택 시 안내 -->
          <div v-if="multiAreaWarning" class="multi-area-notice">
            <Info :size="15" class="notice-icon"/>
            <p class="notice-text">
              일반적으로 하나의 활동은 하나의 영역에만 포함됩니다. 여러 영역에 중복 배치하는 경우는 드문 편이므로, 의도된 구성인지 확인하세요.
            </p>
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
              <button class="btn-cancel-sm" @click="confirmDelete = false">취소</button>
              <button class="btn-delete-confirm" @click="handleDelete">영구 삭제</button>
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
  color: var(--clr-text-subtle);
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.close-btn:hover {
  background-color: #1a2035;
  color: #93afd4;
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
  gap: 16px;
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
  font-size: 13px;
  font-weight: 600;
  color: #7ba3d4;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  margin: 0;
}

.pane-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.selected-count {
  font-size: 13px;
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
  color: var(--clr-text-hint);
}

.field-hint {
  font-size: 14px;
  color: #7ba3d4;
  margin: 0;
  line-height: 1.6;
}

/* 삭제 경고 */
.delete-warning {
  background-color: rgba(239, 68, 68, 0.07);
  border: 1px solid rgba(239, 68, 68, 0.25);
  border-radius: 10px;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.warning-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.warning-icon {
  color: #f87171;
  flex-shrink: 0;
}

.warning-title {
  font-size: 15px;
  font-weight: 600;
  color: #f87171;
}

.warning-body {
  font-size: 14px;
  color: #fca5a5;
  margin: 0;
  line-height: 1.6;
}

.warning-body strong {
  color: #f87171;
  font-weight: 600;
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
.pane-right {
  display: flex;
  flex-direction: column;
}

.empty-hint {
  font-size: 15px;
  color: #7ba3d4;
  line-height: 1.7;
  margin: 0;
}

.chip-scroll {
  display: flex;
  flex-wrap: wrap;
  align-content: flex-start;
  gap: 8px;
  flex: 1;
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

.area-chip {
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

.area-chip:hover {
  border-color: var(--clr-text-subtle);
  color: #93c5fd;
}

.area-chip--on {
  border-color: rgba(59, 91, 219, 0.45);
  background-color: rgba(59, 91, 219, 0.15);
  color: #93c5fd;
}

.area-chip--on:hover {
  background-color: rgba(59, 91, 219, 0.22);
}

/* 복수 영역 안내 */
.multi-area-notice {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  background-color: rgba(251, 191, 36, 0.07);
  border: 1px solid rgba(251, 191, 36, 0.2);
  border-radius: 10px;
  padding: 12px 14px;
  margin-top: 4px;
}

.notice-icon {
  color: #fbbf24;
  flex-shrink: 0;
  margin-top: 1px;
}

.notice-text {
  font-size: 14px;
  color: #fcd34d;
  margin: 0;
  line-height: 1.6;
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
  padding: 6px 14px;
  border-radius: 8px;
  background-color: rgba(239, 68, 68, 0.85);
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
  color: #93afd4;
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
