<script setup>
import {onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {invoke} from '@tauri-apps/api/core'
import {open, save} from '@tauri-apps/plugin-dialog'
import {getVersion} from '@tauri-apps/api/app'
import {openUrl} from '@tauri-apps/plugin-opener'
import {useProjectStore} from '../stores/project'

const router = useRouter()
const project = useProjectStore()
const error = ref('')

const currentVersion = ref('')
const showUpdateModal = ref(false)
const updateStatus = ref('idle') // 'idle' | 'checking' | 'latest' | 'found' | 'error'
const latestVersion = ref('')
const releaseUrl = ref('')

onMounted(async () => {
  currentVersion.value = await getVersion()
})

async function handleNew() {
  error.value = ''
  const path = await save({
    title: '새 학생부 파일 위치 선택',
    defaultPath: 'school_record.db',
    filters: [{name: 'SQLite DB', extensions: ['db']}],
  })
  if (!path) return
  try {
    await invoke('new_project', {path})
    project.setProject(path)
    router.push('/workspace')
  } catch (e) {
    error.value = String(e)
  }
}

async function handleOpen() {
  error.value = ''
  const path = await open({
    title: '기존 학생부 파일 선택',
    filters: [{name: 'SQLite DB', extensions: ['db']}],
    multiple: false,
  })
  if (!path) return
  try {
    await invoke('open_project', {path})
    project.setProject(path)
    router.push('/workspace')
  } catch (e) {
    error.value = String(e)
  }
}

async function checkUpdate() {
  showUpdateModal.value = true
  updateStatus.value = 'checking'
  try {
    const res = await fetch('https://api.github.com/repos/itmir913/School-Record-App/releases/latest')
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const data = await res.json()
    latestVersion.value = data.tag_name
    releaseUrl.value = 'https://github.com/itmir913/School-Record-App/releases/latest'
    const tag = data.tag_name.replace(/^v/, '')
    updateStatus.value = tag !== currentVersion.value ? 'found' : 'latest'
  } catch {
    updateStatus.value = 'error'
  }
}

function closeUpdateModal() {
  showUpdateModal.value = false
  updateStatus.value = 'idle'
}
</script>

<template>
  <div class="activity-section-wrapper">
    <div class="page">
      <!-- ambient glow -->
      <div class="glow"/>

      <!-- 플로팅 카드 -->
      <div class="card">

        <!-- 로고 -->
        <div class="logo-wrap">
          <div class="logo-icon">
            <svg width="34" height="34" viewBox="0 0 24 24" fill="none"
                 stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
            </svg>
            <span class="logo-badge"/>
          </div>
          <div class="logo-text">
            <h1>학교생활기록부 작성 프로그램</h1>
            <p>학생부를 체계적으로 작성하기 위한 유틸리티 프로그램</p>
          </div>
        </div>

        <!-- 구분선 -->
        <div class="divider">
          <div class="divider-line"/>
          <span class="divider-dot"/>
          <div class="divider-line"/>
        </div>

        <!-- 버튼 -->
        <div class="actions">
          <button class="btn-primary" @click="handleNew">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 4v16m8-8H4"/>
            </svg>
            새 학생부 만들기
            <svg class="arrow" width="17" height="17" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 5l7 7-7 7"/>
            </svg>
          </button>

          <button class="btn-secondary" @click="handleOpen">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path
                  d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/>
            </svg>
            기존 파일 열기
            <svg class="arrow" width="17" height="17" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 5l7 7-7 7"/>
            </svg>
          </button>

          <button class="btn-update" @click="checkUpdate">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="1 4 1 10 7 10"/>
              <polyline points="23 20 23 14 17 14"/>
              <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15"/>
            </svg>
            업데이트 확인
          </button>
        </div>

        <!-- 에러 -->
        <transition name="err">
          <div v-if="error" class="error-box">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                 style="flex-shrink:0; margin-top:1px;">
              <path
                  d="M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
            </svg>
            {{ error }}
          </div>
        </transition>

        <p class="version">v{{ currentVersion }}</p>
      </div>
    </div>

    <!-- 업데이트 모달 -->
    <transition name="modal">
      <div v-if="showUpdateModal" class="overlay">
        <div class="modal">
          <div class="modal-header">
            <div>
              <h2>업데이트 확인</h2>
              <p>현재 버전 v{{ currentVersion }}</p>
            </div>
            <button class="close-btn" @click="closeUpdateModal">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M6 18L18 6M6 6l12 12"/>
              </svg>
            </button>
          </div>
          <div class="update-body">

            <!-- 확인 중 -->
            <div v-if="updateStatus === 'checking'" class="update-checking">
              <div class="spinner"/>
              <p>최신 버전을 확인하는 중…</p>
            </div>

            <!-- 최신 버전 -->
            <div v-else-if="updateStatus === 'latest'" class="update-state update-latest">
              <svg width="25" height="25" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M20 6L9 17l-5-5"/>
              </svg>
              <div>
                <p class="state-title">최신 버전입니다</p>
                <p class="state-desc">현재 사용 중인 버전이 최신입니다.</p>
              </div>
            </div>

            <!-- 새 버전 있음 -->
            <div v-else-if="updateStatus === 'found'" class="update-state update-found">
              <svg width="25" height="25" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path
                    d="M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
              </svg>
              <div>
                <p class="state-title">새 버전이 있습니다 — {{ latestVersion }}</p>
                <p class="state-desc">GitHub에서 최신 버전을 다운로드할 수 있습니다.</p>
              </div>
            </div>

            <!-- 오류 -->
            <div v-else-if="updateStatus === 'error'" class="update-state update-error">
              <svg width="25" height="25" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 8v4m0 4h.01"/>
              </svg>
              <div>
                <p class="state-title">확인에 실패했습니다</p>
                <p class="state-desc">인터넷 연결을 확인한 후 다시 시도해 주세요.</p>
              </div>
            </div>

            <!-- 다운로드 버튼 (새 버전일 때만) -->
            <button v-if="updateStatus === 'found'" class="btn-download" @click="openUrl(releaseUrl)">
              <svg width="17" height="17" viewBox="0 0 24 24" fill="none"
                   stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/>
              </svg>
              GitHub에서 다운로드
            </button>

          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* ── 전체 페이지 ── */
.page {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  overflow: hidden;
  background-color: #080b14;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

/* 배경 glow */
.glow {
  position: absolute;
  inset: 0;
  background: radial-gradient(ellipse 60% 50% at 50% 50%, rgba(59, 91, 219, 0.12), transparent);
  pointer-events: none;
}

/* ── 카드 ── */
.card {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 440px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  padding: 40px 36px 32px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.03);
}

/* ── 로고 ── */
.logo-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  margin-bottom: 32px;
}

.logo-icon {
  position: relative;
  width: 68px;
  height: 68px;
  border-radius: 20px;
  background: linear-gradient(135deg, #3b5bdb, #4c6ef5);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 0 1px rgba(251, 191, 36, 0.2), 0 8px 32px rgba(59, 91, 219, 0.35);
}

.logo-badge {
  position: absolute;
  top: -5px;
  right: -5px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background-color: #fbbf24;
  border: 2px solid #0e1220;
  box-shadow: 0 0 8px rgba(251, 191, 36, 0.5);
}

.logo-text {
  text-align: center;
}

.logo-text h1 {
  font-size: 21px;
  font-weight: 700;
  color: #e2e8f0;
  letter-spacing: -0.02em;
  margin: 0;
}

.logo-text p {
  font-size: 15px;
  color: var(--clr-text-hint);
  margin: 5px 0 0;
}

/* ── 구분선 ── */
.divider {
  display: flex;
  align-items: center;
  margin-bottom: 26px;
}

.divider-line {
  flex: 1;
  height: 1px;
  background-color: #1a2035;
}

.divider-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: #fbbf24;
  opacity: 0.5;
  margin: 0 12px;
}

/* ── 버튼 ── */
.actions {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.btn-primary,
.btn-secondary {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 15px 20px;
  border-radius: 14px;
  font-size: 17px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: background-color 0.15s, transform 0.1s;
  text-align: left;
}

.btn-primary:active,
.btn-secondary:active {
  transform: scale(0.98);
}

.btn-primary {
  background-color: #3b5bdb;
  color: #ffffff;
}

.btn-primary:hover {
  background-color: #4c6ef5;
}

.btn-secondary {
  background-color: #131c30;
  border-color: #2e3f60;
  color: #7ba3d4;
}

.btn-secondary:hover {
  background-color: #1a2640;
  border-color: #4a6090;
}

.arrow {
  margin-left: auto;
  opacity: 0;
  transition: opacity 0.15s, transform 0.15s;
}

.btn-primary:hover .arrow,
.btn-secondary:hover .arrow {
  opacity: 1;
  transform: translateX(2px);
}

.btn-update {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 10px;
  background: none;
  border: 1px solid #2e3f60;
  border-radius: 10px;
  font-size: 15px;
  color: var(--clr-text-hint);
  cursor: pointer;
  margin-top: 2px;
  transition: color 0.15s, border-color 0.15s, background-color 0.15s;
}

.btn-update:hover {
  color: #a8c4e8;
  border-color: #4a6090;
  background-color: #0d1525;
}

/* ── 에러 ── */
.error-box {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-top: 18px;
  padding: 14px 18px;
  border-radius: 12px;
  background-color: #2a1020;
  border: 1px solid #4a1a28;
  font-size: 15px;
  color: #fca5a5;
  line-height: 1.5;
}

.err-enter-from, .err-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.err-enter-active, .err-leave-active {
  transition: all 0.2s;
}

/* ── 버전 ── */
.version {
  margin-top: 24px;
  text-align: center;
  font-size: 13px;
  color: var(--clr-text-hint);
}

/* ── 모달 오버레이 ── */
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
  max-width: 560px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 24px;
  padding: 34px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7);
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 24px;
}

.modal-header h2 {
  font-size: 19px;
  font-weight: 600;
  color: #e2e8f0;
  margin: 0;
}

.modal-header p {
  font-size: 14px;
  color: var(--clr-text-hint);
  margin: 5px 0 0;
}

.close-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--clr-text-hint);
  padding: 8px;
  border-radius: 10px;
  display: flex;
  transition: background-color 0.15s, color 0.15s;
}

.close-btn:hover {
  background-color: #1a2035;
  color: #7ba3d4;
}

/* ── 업데이트 모달 바디 ── */
.update-body {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.update-checking {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  padding: 30px 0;
  color: var(--clr-text-hint);
  font-size: 16px;
}

.spinner {
  width: 30px;
  height: 30px;
  border: 2px solid #1a2035;
  border-top-color: #4c6ef5;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.update-state {
  display: flex;
  align-items: flex-start;
  gap: 15px;
  padding: 18px 20px;
  border-radius: 14px;
  border: 1px solid;
}

.update-latest {
  background-color: #0a1f14;
  border-color: #1a4a2a;
  color: #4ade80;
}

.update-found {
  background-color: #1c1508;
  border-color: #4a3800;
  color: #fbbf24;
}

.update-error {
  background-color: #1c0a0a;
  border-color: #4a1a1a;
  color: #fca5a5;
}

.update-state svg {
  flex-shrink: 0;
  margin-top: 1px;
}

.state-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 5px;
}

.state-desc {
  font-size: 14px;
  opacity: 0.75;
  margin: 0;
  line-height: 1.5;
}

.btn-download {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  padding: 14px 20px;
  border-radius: 12px;
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  border: none;
  background-color: #3b5bdb;
  color: #ffffff;
  transition: background-color 0.15s, transform 0.1s;
}

.btn-download:hover {
  background-color: #4c6ef5;
}

.btn-download:active {
  transform: scale(0.98);
}

.modal-enter-from, .modal-leave-to {
  opacity: 0;
}

.modal-enter-active, .modal-leave-active {
  transition: opacity 0.2s;
}
</style>
