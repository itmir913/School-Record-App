<script setup>
import {ref, watch} from 'vue'
import {AlertTriangle, Trash2, X} from 'lucide-vue-next'

const props = defineProps({
  mode: {type: String, default: 'add'}, // 'add' | 'edit'
  student: {type: Object, default: null},
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
  emit('saved', {
    grade: Number(grade.value),
    classNum: Number(classNum.value),
    number: Number(number.value),
    name: name.value.trim(),
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
        <h2 class="modal-title">{{ mode === 'add' ? '학생 추가' : '학생 수정' }}</h2>
        <button class="close-btn" @click="emit('close')">
          <X :size="18"/>
        </button>
      </div>

      <!-- 바디 -->
      <div class="modal-body">

        <div class="row-fields">
          <div class="field field-sm">
            <label class="field-label">학년 <span class="required">*</span></label>
            <input
                v-model="grade"
                class="field-input"
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
                class="field-input"
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
                class="field-input"
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
              class="field-input"
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
            이 학생을 삭제하면 <strong>모든 활동 기록이 함께 삭제</strong>되며 복구할 수 없습니다.
          </p>
        </div>

        <p v-if="error" class="form-error">{{ error }}</p>
      </div>

      <!-- 푸터 -->
      <div class="modal-footer">
        <div class="footer-left">
          <template v-if="mode === 'edit'">
            <button v-if="!confirmDelete" class="btn-delete" @click="handleDelete">
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
  max-width: 460px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7);
  overflow: hidden;
}

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
  color: #5a7aaa;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.close-btn:hover {
  background-color: #1a2035;
  color: #93afd4;
}

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
  color: #3d5580;
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

/* 푸터 */
.modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px 20px;
  border-top: 1px solid #1a2035;
  margin-top: 8px;
}

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
  padding: 8px 14px;
  border-radius: 8px;
  border: 1px solid rgba(239, 68, 68, 0.3);
  background: none;
  color: #f87171;
  font-size: 15px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-delete:hover {
  background-color: rgba(239, 68, 68, 0.08);
}

.confirm-row {
  display: flex;
  gap: 8px;
}

.btn-cancel-sm {
  padding: 8px 14px;
  border-radius: 8px;
  border: 1px solid #1a2035;
  background: none;
  color: #7ba3d4;
  font-size: 15px;
  cursor: pointer;
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

.btn-cancel {
  padding: 10px 20px;
  border-radius: 10px;
  border: 1px solid #1a2035;
  background: none;
  color: #7ba3d4;
  font-size: 16px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-cancel:hover {
  background-color: #1a2035;
}

.btn-submit {
  padding: 10px 24px;
  border-radius: 10px;
  border: none;
  background-color: #3b5bdb;
  color: white;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s;
  box-shadow: 0 4px 16px rgba(59, 91, 219, 0.2);
}

.btn-submit:hover {
  background-color: #4c6ef5;
}
</style>
