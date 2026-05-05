<script setup>
import {ref} from 'vue'
import {AlertTriangle, KeyRound, Shield, ShieldOff} from 'lucide-vue-next'
import {useConfigStore} from '../stores/configStore'
import PasswordModal from '../components/PasswordModal.vue'

const config = useConfigStore()

const showPasswordModal = ref(false)
const passwordModalMode = ref('setup')
const passwordError = ref('')
const passwordLoading = ref(false)
const statusEncryptMessage = ref('')
const confirmEncryptDisable = ref(false)
const disableEncryptLoading = ref(false)

function openSetup() {
  passwordModalMode.value = 'setup'
  passwordError.value = ''
  statusEncryptMessage.value = ''
  showPasswordModal.value = true
}

function openChange() {
  passwordModalMode.value = 'change'
  passwordError.value = ''
  statusEncryptMessage.value = ''
  showPasswordModal.value = true
}

async function handleDisable() {
  if (!confirmEncryptDisable.value) {
    confirmEncryptDisable.value = true
    return
  }
  confirmEncryptDisable.value = false
  disableEncryptLoading.value = true
  statusEncryptMessage.value = ''
  try {
    await config.disableEncryption()
    statusEncryptMessage.value = '암호화가 비활성화되었습니다.'
  } catch (e) {
    statusEncryptMessage.value = '오류: ' + String(e)
  } finally {
    disableEncryptLoading.value = false
  }
}

async function handlePasswordSubmit(payload) {
  passwordError.value = ''
  passwordLoading.value = true
  try {
    if (passwordModalMode.value === 'setup') {
      await config.enableEncryption(payload.password)
      statusEncryptMessage.value = '암호화가 활성화되었습니다.'
    } else {
      await config.changeEncryptionPassword(payload.oldPassword, payload.newPassword)
      statusEncryptMessage.value = '비밀번호가 변경되었습니다.'
    }
    showPasswordModal.value = false
  } catch (e) {
    passwordError.value = String(e)
  } finally {
    passwordLoading.value = false
  }
}
</script>

<template>
  <div class="section">
    <div class="section-header">
      <div>
        <h2 class="section-title">설정(Settings)</h2>
        <p class="section-desc">파일 및 보안 설정을 관리합니다.</p>
      </div>
    </div>

    <div class="section-body">

      <!-- 암호화 설정 카드 -->
      <div class="settings-card">
        <div class="card-header">
          <div class="card-icon" :class="config.encryptionEnabled ? 'icon-enabled' : 'icon-disabled'">
            <Shield v-if="config.encryptionEnabled" :size="20"/>
            <ShieldOff v-else :size="20"/>
          </div>
          <div>
            <h3 class="card-title">데이터 암호화</h3>
            <p class="card-desc">학생 이름과 생기부 내용을 암호화합니다.</p>
          </div>
          <div class="card-badge" :class="config.encryptionEnabled ? 'badge-on' : 'badge-off'">
            {{ config.encryptionEnabled ? '활성화' : '비활성화' }}
          </div>
        </div>

        <!-- 경고 문구 -->
        <div class="warning-box">
          <AlertTriangle :size="14" class="warning-icon"/>
          <span>암호화 활성화 시 비밀번호를 분실하면 데이터를 복구할 수 없습니다.</span>
        </div>

        <!-- 버튼 -->
        <div class="card-actions">
          <button v-if="!config.encryptionEnabled" class="btn-enable" @click="openSetup">
            <Shield :size="16"/>
            암호화 활성화
          </button>
          <template v-else>
            <button class="btn-change" :disabled="disableEncryptLoading" @click="openChange">
              <KeyRound :size="16"/>
              비밀번호 변경
            </button>
            <button v-if="!confirmEncryptDisable" class="btn-disable" :disabled="disableEncryptLoading" @click="handleDisable">
              <ShieldOff :size="16"/>
              암호화 비활성화
            </button>
            <div v-else class="disable-confirm-row">
              <div class="disable-warning">
                <AlertTriangle :size="14" class="disable-warning-icon"/>
                <span class="disable-warning-title">암호화를 정말 비활성화하시겠습니까?</span>
              </div>
              <div class="disable-confirm-btns">
                <button class="btn-cancel-sm" :disabled="disableEncryptLoading" @click="confirmEncryptDisable = false">취소</button>
                <button class="btn-disable-confirm" :disabled="disableEncryptLoading" @click="handleDisable">
                  {{ disableEncryptLoading ? '처리 중…' : '비활성화' }}
                </button>
              </div>
            </div>
          </template>
        </div>

        <!-- 상태 메시지 -->
        <transition name="fade">
          <p v-if="statusEncryptMessage" class="status-msg"
             :class="statusEncryptMessage.startsWith('오류') ? 'status-error' : 'status-ok'">
            {{ statusEncryptMessage }}
          </p>
        </transition>
      </div>
    </div>

    <!-- 비밀번호 모달 -->
    <PasswordModal
        v-if="showPasswordModal"
        :mode="passwordModalMode"
        :error="passwordError"
        :loading="passwordLoading"
        @submit="handlePasswordSubmit"
        @cancel="showPasswordModal = false"
    />
  </div>
</template>

<style scoped>
.section {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  box-sizing: border-box;
}

.section-body {
  flex: 1;
  overflow-y: auto;
  padding: 32px 40px 48px;
}

.section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 36px 40px;
  border-bottom: 1px solid #1a2035;
  flex-shrink: 0;
  gap: 16px;
}

.section-title {
  font-size: 22px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0 0 6px;
}

.section-desc {
  font-size: 16px;
  color: #7ba3d4;
  margin: 0;
}

.settings-card {
  background-color: #0e1524;
  border: 1px solid #1e2d45;
  border-radius: 16px;
  padding: 24px;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
}

.card-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 12px;
  flex-shrink: 0;
}

.icon-enabled {
  background-color: rgba(16, 185, 129, 0.12);
  border: 1px solid rgba(16, 185, 129, 0.3);
  color: #34d399;
}

.icon-disabled {
  background-color: rgba(100, 116, 139, 0.12);
  border: 1px solid rgba(100, 116, 139, 0.3);
  color: #64748b;
}

.card-title {
  font-size: 16px;
  font-weight: 600;
  color: #e2e8f0;
  margin: 0 0 4px;
}

.card-desc {
  font-size: 14px;
  color: var(--clr-text-hint);
  margin: 0;
}

.card-badge {
  margin-left: auto;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  flex-shrink: 0;
}

.badge-on {
  background-color: rgba(16, 185, 129, 0.12);
  border: 1px solid rgba(16, 185, 129, 0.3);
  color: #34d399;
}

.badge-off {
  background-color: rgba(100, 116, 139, 0.1);
  border: 1px solid rgba(100, 116, 139, 0.25);
  color: #64748b;
}

.warning-box {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 14px;
  border-radius: 10px;
  background-color: rgba(245, 158, 11, 0.07);
  border: 1px solid rgba(245, 158, 11, 0.2);
  font-size: 13px;
  color: #fbbf24;
  line-height: 1.5;
  margin-bottom: 18px;
}

.warning-icon {
  flex-shrink: 0;
  margin-top: 1px;
}

.card-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.btn-enable,
.btn-change,
.btn-disable {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 18px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.15s, transform 0.1s;
  border: 1px solid transparent;
}

.btn-enable:active, .btn-change:active, .btn-disable:active {
  transform: scale(0.97);
}

.btn-enable {
  background-color: rgba(16, 185, 129, 0.15);
  border-color: rgba(16, 185, 129, 0.35);
  color: #34d399;
}

.btn-enable:hover {
  background-color: rgba(16, 185, 129, 0.25);
}

.btn-change {
  background-color: rgba(59, 91, 219, 0.15);
  border-color: rgba(59, 91, 219, 0.35);
  color: #6ea8fe;
}

.btn-change:hover {
  background-color: rgba(59, 91, 219, 0.25);
}

.btn-disable {
  background-color: rgba(239, 68, 68, 0.1);
  border-color: rgba(239, 68, 68, 0.3);
  color: #fca5a5;
}

.btn-disable:hover {
  background-color: rgba(239, 68, 68, 0.18);
}

.status-msg {
  margin: 14px 0 0;
  font-size: 14px;
  font-weight: 500;
}

.status-ok {
  color: #34d399;
}

.status-error {
  color: #fca5a5;
}

.disable-confirm-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 10px;
}

.disable-warning {
  display: flex;
  align-items: center;
  gap: 8px;
  background-color: rgba(239, 68, 68, 0.07);
  border: 1px solid rgba(239, 68, 68, 0.25);
  border-radius: 10px;
  padding: 9px 18px;
}

.disable-warning-icon {
  color: #f87171;
  flex-shrink: 0;
}

.disable-warning-title {
  font-size: 14px;
  font-weight: 600;
  color: #f87171;
}

.disable-confirm-btns {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-cancel-sm {
  padding: 9px 18px;
  border-radius: 8px;
  border: 1px solid rgba(100, 116, 139, 0.4);
  background: transparent;
  color: #94a3b8;
  font-size: 14px;
  cursor: pointer;
  box-sizing: border-box;
}

.btn-cancel-sm:hover {
  background-color: rgba(100, 116, 139, 0.12);
}

.btn-disable-confirm {
  padding: 9px 18px;
  border-radius: 8px;
  border: 1px solid transparent;
  background-color: #ef4444;
  color: white;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s;
  box-sizing: border-box;
}

.btn-disable-confirm:hover {
  background-color: #dc2626;
}

.fade-enter-from, .fade-leave-to {
  opacity: 0;
}

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.3s;
}
</style>
