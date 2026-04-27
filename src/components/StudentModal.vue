<script setup>
import {ref, watch} from 'vue'
import {AlertTriangle, Trash2} from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({
  mode: {type: String, default: 'add'}, // 'add' | 'edit'
  student: {type: Object, default: null},
  submitting: {type: Boolean, default: false},
})

const emit = defineEmits(['close', 'saved', 'deleted'])

const grade = ref('')
const classNum = ref('')
const number = ref('')
const name = ref('')
const error = ref('')
const confirmDelete = ref(false)

watch(
    () => props.student,
    (s) => {
      grade.value = s ? String(s.grade) : ''
      classNum.value = s ? String(s.class_num) : ''
      number.value = s ? String(s.number) : ''
      name.value = s ? s.name : ''
      error.value = ''
      confirmDelete.value = false
    },
    {immediate: true}
)

function validate() {
  if (!grade.value || isNaN(Number(grade.value)) || Number(grade.value) < 1) {
    error.value = '올바른 학년을 입력해주세요.'
    return false
  }
  if (!classNum.value || isNaN(Number(classNum.value)) || Number(classNum.value) < 1) {
    error.value = '올바른 반을 입력해주세요.'
    return false
  }
  if (!number.value || isNaN(Number(number.value)) || Number(number.value) < 1) {
    error.value = '올바른 번호를 입력해주세요.'
    return false
  }
  if (!name.value.trim()) {
    error.value = '이름을 입력해주세요.'
    return false
  }
  return true
}

function submit() {
  if (!validate()) return
  error.value = ''
  emit('saved', {
    grade: Number(grade.value),
    classNum: Number(classNum.value),
    number: Number(number.value),
    name: name.value.trim(),
  })
}

function setServerError(msg) {
  error.value = msg
}

defineExpose({ setServerError })

function handleDelete() {
  if (!confirmDelete.value) {
    confirmDelete.value = true
    return
  }
  emit('deleted')
}
</script>

<template>
  <BaseModal
      :title="mode === 'add' ? '학생 추가' : '학생 수정'"
      max-width="460px"
      @close="emit('close')"
  >
    <!-- 바디 -->
    <div class="modal-body">

      <div class="row-fields">
        <div class="field field-sm">
          <label class="field-label">학년 <span class="required">*</span></label>
          <input
              v-model="grade"
              class="ui-input field-input"
              type="number"
              min="1"
              placeholder="3"
              @keydown.enter="submit"
          />
        </div>
        <div class="field field-sm">
          <label class="field-label">반 <span class="required">*</span></label>
          <input
              v-model="classNum"
              class="ui-input field-input"
              type="number"
              min="1"
              placeholder="2"
              @keydown.enter="submit"
          />
        </div>
        <div class="field field-sm">
          <label class="field-label">번호 <span class="required">*</span></label>
          <input
              v-model="number"
              class="ui-input field-input"
              type="number"
              min="1"
              placeholder="15"
              @keydown.enter="submit"
          />
        </div>
      </div>

      <div class="field">
        <label class="field-label">이름 <span class="required">*</span></label>
        <input
            v-model="name"
            class="ui-input field-input"
            placeholder="홍길동"
            @keydown.enter="submit"
        />
      </div>

      <!-- 삭제 경고 -->
      <div v-if="mode === 'edit' && confirmDelete" class="delete-warning">
        <div class="warning-header">
          <AlertTriangle :size="16" class="warning-icon"/>
          <span class="warning-title">정말 삭제하시겠습니까?</span>
        </div>
        <p class="warning-body">
          이 학생을 삭제하면 <strong>모든 활동 기록과 스냅샷 정보가 함께 삭제</strong>되며, 이후 스냅샷을 이용한 복구도 불가능합니다.
        </p>
      </div>

      <p v-if="error" class="msg-error">{{ error }}</p>
    </div>

    <!-- 푸터 -->
    <template #footer>
      <div class="footer-left">
        <template v-if="mode === 'edit'">
          <button v-if="!confirmDelete" class="btn-danger btn-delete" @click="handleDelete">
            <Trash2 :size="15"/>
            삭제
          </button>
          <div v-else class="confirm-row">
            <button class="btn-secondary btn-cancel-sm" @click="confirmDelete = false">취소</button>
            <button class="btn-delete-confirm" @click="handleDelete">영구 삭제</button>
          </div>
        </template>
      </div>

      <div class="footer-right">
        <button class="btn-secondary" @click="emit('close')">취소</button>
        <button class="btn-primary" :disabled="submitting" @click="submit">
          {{ mode === 'add' ? '추가' : '저장' }}
        </button>
      </div>
    </template>
  </BaseModal>
</template>

<style scoped>
.modal-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px 24px 8px;
}

.row-fields {
  display: flex;
  gap: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
}

.field-sm {
  flex: 1;
  min-width: 0;
}

.field-label {
  font-size: 15px;
  font-weight: 600;
  color: #93afd4;
}

.required {
  color: #f87171;
}

.field-input::placeholder {
  color: var(--clr-text-hint);
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

/* 푸터 */
.footer-left,
.footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-delete {
  display: flex;
  align-items: center;
  gap: 6px;
}

.confirm-row {
  display: flex;
  gap: 8px;
}

.btn-cancel-sm {
  padding: 8px 14px;
  border-radius: 8px;
  font-size: 15px;
}

.btn-delete-confirm {
  padding: 8px 14px;
  border-radius: 8px;
  border: none;
  background-color: #ef4444;
  color: white;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-delete-confirm:hover {
  background-color: #dc2626;
}
</style>
