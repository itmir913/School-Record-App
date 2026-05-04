<script setup>
import { ref, computed, watch } from 'vue'
import { Lock, Eye, EyeOff, AlertTriangle } from 'lucide-vue-next'

const props = defineProps({
  // 'unlock' | 'setup' | 'change'
  mode: { type: String, required: true },
  error: { type: String, default: '' },
  loading: { type: Boolean, default: false },
})

const emit = defineEmits(['submit', 'cancel'])

const password = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const showPassword = ref(false)
const showNewPassword = ref(false)
const localError = ref('')

const isSetupMode = computed(() => props.mode === 'setup')
const isChangeMode = computed(() => props.mode === 'change')

const title = computed(() => {
  if (props.mode === 'unlock') return '비밀번호 확인'
  if (props.mode === 'setup') return '암호화 비밀번호 설정'
  return '비밀번호 변경'
})

const submitLabel = computed(() => {
  if (props.mode === 'unlock') return '잠금 해제'
  if (props.mode === 'setup') return '암호화 활성화'
  return '비밀번호 변경'
})

watch(() => props.error, (val) => {
  if (val) localError.value = val
})

function validate() {
  localError.value = ''
  if (!password.value) {
    localError.value = '비밀번호를 입력해주세요.'
    return false
  }

  if (isSetupMode.value) {
    if (password.value !== confirmPassword.value) {
      localError.value = '비밀번호와 확인 비밀번호가 일치하지 않습니다.'
      return false
    }
  }

  if (isChangeMode.value) {
    if (!newPassword.value) {
      localError.value = '새 비밀번호를 입력해주세요.'
      return false
    }
    if (newPassword.value !== confirmPassword.value) {
      localError.value = '새 비밀번호와 확인 비밀번호가 일치하지 않습니다.'
      return false
    }
  }

  return true
}

function handleSubmit() {
  if (!validate()) return
  if (props.mode === 'unlock') {
    emit('submit', { password: password.value })
  } else if (props.mode === 'setup') {
    emit('submit', { password: password.value })
  } else {
    emit('submit', { oldPassword: password.value, newPassword: newPassword.value })
  }
}

function handleCancel() {
  emit('cancel')
}
</script>

<template>
  <div class="overlay">
    <div class="modal">
      <div class="modal-header">
        <div class="header-icon">
          <Lock :size="20"/>
        </div>
        <div>
          <h2>{{ title }}</h2>
          <p v-if="mode === 'unlock'">이 파일은 암호화되어 있습니다.</p>
          <p v-else-if="mode === 'setup'">암호화할 비밀번호를 설정합니다.</p>
          <p v-else>현재 비밀번호와 새 비밀번호를 입력합니다.</p>
        </div>
      </div>

      <!-- 비밀번호 분실 경고 (setup 모드) -->
      <div v-if="mode === 'setup'" class="warning-box">
        <AlertTriangle :size="16" class="warning-icon"/>
        <span>비밀번호를 분실하면 데이터를 복구할 수 없습니다. 반드시 안전한 곳에 보관하세요.</span>
      </div>

      <div class="form">
        <!-- 현재/기존 비밀번호 -->
        <div class="field">
          <label>{{ mode === 'change' ? '현재 비밀번호' : '비밀번호' }}</label>
          <div class="input-wrap">
            <input
              :type="showPassword ? 'text' : 'password'"
              v-model="password"
              :placeholder="mode === 'change' ? '현재 비밀번호' : '비밀번호 입력'"
              @keydown.enter="handleSubmit"
              autofocus
            />
            <button type="button" class="eye-btn" @click="showPassword = !showPassword">
              <Eye v-if="!showPassword" :size="16"/>
              <EyeOff v-else :size="16"/>
            </button>
          </div>
        </div>

        <!-- 새 비밀번호 (change 모드) -->
        <div v-if="mode === 'change'" class="field">
          <label>새 비밀번호</label>
          <div class="input-wrap">
            <input
              :type="showNewPassword ? 'text' : 'password'"
              v-model="newPassword"
              placeholder="새 비밀번호 입력"
              @keydown.enter="handleSubmit"
            />
            <button type="button" class="eye-btn" @click="showNewPassword = !showNewPassword">
              <Eye v-if="!showNewPassword" :size="16"/>
              <EyeOff v-else :size="16"/>
            </button>
          </div>
        </div>

        <!-- 비밀번호 확인 (setup/change 모드) -->
        <div v-if="mode === 'setup' || mode === 'change'" class="field">
          <label>{{ mode === 'setup' ? '비밀번호 확인' : '새 비밀번호 확인' }}</label>
          <div class="input-wrap">
            <input
              type="password"
              v-model="confirmPassword"
              :placeholder="mode === 'setup' ? '비밀번호 재입력' : '새 비밀번호 재입력'"
              @keydown.enter="handleSubmit"
            />
          </div>
        </div>

        <!-- 오류 메시지 -->
        <transition name="err">
          <div v-if="localError" class="error-box">
            <AlertTriangle :size="15" style="flex-shrink:0; margin-top:1px;"/>
            {{ localError }}
          </div>
        </transition>
      </div>

      <div class="actions">
        <button class="btn-cancel" @click="handleCancel" :disabled="loading">
          {{ mode === 'unlock' ? '뒤로 가기' : '취소' }}
        </button>
        <button class="btn-submit" @click="handleSubmit" :disabled="loading">
          <span v-if="loading" class="spinner"/>
          <span v-else>{{ submitLabel }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(4, 6, 12, 0.8);
  backdrop-filter: blur(6px);
}

.modal {
  width: 100%;
  max-width: 420px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  padding: 32px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7);
}

.modal-header {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  margin-bottom: 20px;
}

.header-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background-color: rgba(59, 91, 219, 0.15);
  border: 1px solid rgba(59, 91, 219, 0.3);
  color: #6ea8fe;
  flex-shrink: 0;
  margin-top: 2px;
}

.modal-header h2 {
  font-size: 18px;
  font-weight: 600;
  color: #e2e8f0;
  margin: 0 0 4px;
}

.modal-header p {
  font-size: 14px;
  color: var(--clr-text-hint);
  margin: 0;
}

.warning-box {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px 14px;
  border-radius: 10px;
  background-color: rgba(245, 158, 11, 0.08);
  border: 1px solid rgba(245, 158, 11, 0.25);
  font-size: 13px;
  color: #fbbf24;
  line-height: 1.5;
  margin-bottom: 18px;
}

.warning-icon {
  flex-shrink: 0;
  margin-top: 1px;
}

.form {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-bottom: 22px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field label {
  font-size: 13px;
  font-weight: 500;
  color: #94a3b8;
}

.input-wrap {
  position: relative;
}

.input-wrap input {
  width: 100%;
  padding: 10px 40px 10px 14px;
  background-color: #0b1022;
  border: 1px solid #2e3f60;
  border-radius: 10px;
  color: #e2e8f0;
  font-size: 15px;
  outline: none;
  transition: border-color 0.15s;
  box-sizing: border-box;
}

.input-wrap input:focus {
  border-color: #4c6ef5;
}

.eye-btn {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  color: var(--clr-text-hint);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  transition: color 0.15s;
}

.eye-btn:hover {
  color: #7ba3d4;
}

.error-box {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 11px 14px;
  border-radius: 10px;
  background-color: #2a1020;
  border: 1px solid #4a1a28;
  font-size: 13px;
  color: #fca5a5;
  line-height: 1.5;
}

.err-enter-from, .err-leave-to { opacity: 0; transform: translateY(4px); }
.err-enter-active, .err-leave-active { transition: all 0.2s; }

.actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.btn-cancel {
  padding: 10px 20px;
  border-radius: 10px;
  background: none;
  border: 1px solid #2e3f60;
  color: #94a3b8;
  font-size: 15px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.btn-cancel:hover:not(:disabled) {
  background-color: #1a2035;
  color: #e2e8f0;
}

.btn-submit {
  padding: 10px 24px;
  border-radius: 10px;
  background-color: #3b5bdb;
  border: none;
  color: #ffffff;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  transition: background-color 0.15s;
  min-width: 100px;
  justify-content: center;
}

.btn-submit:hover:not(:disabled) {
  background-color: #4c6ef5;
}

.btn-submit:disabled, .btn-cancel:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
