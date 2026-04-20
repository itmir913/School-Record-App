<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { save, open } from '@tauri-apps/plugin-dialog'
import { useProjectStore } from '../stores/project'

const router = useRouter()
const project = useProjectStore()
const error = ref('')
const showGuide = ref(false)

async function handleNew() {
  error.value = ''
  const path = await save({
    title: '새 학생부 파일 위치 선택',
    defaultPath: 'school_record.db',
    filters: [{ name: 'SQLite DB', extensions: ['db'] }],
  })
  if (!path) return
  try {
    await invoke('new_project', { path })
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
    filters: [{ name: 'SQLite DB', extensions: ['db'] }],
    multiple: false,
  })
  if (!path) return
  try {
    await invoke('open_project', { path })
    project.setProject(path)
    router.push('/workspace')
  } catch (e) {
    error.value = String(e)
  }
}

const guideSteps = [
  { num: '01', title: 'Area 영역 설정',  desc: '자율활동, 동아리활동 등 생기부 대분류를 Area로 등록합니다.',       accent: '#60a5fa' },
  { num: '02', title: 'Activity 등록',   desc: '각 Area 안에 세부 활동 항목을 구성합니다.',                       accent: '#818cf8' },
  { num: '03', title: '학생 정보 입력',  desc: '학년·반·번호·이름으로 학생을 등록합니다.',                        accent: '#38bdf8' },
  { num: '04', title: '기록 작성',       desc: '그리드 뷰에서 학생별 활동 내용을 직접 입력하고 저장합니다.',       accent: '#fbbf24' },
]
</script>

<template>
  <div class="page">
    <!-- ambient glow -->
    <div class="glow" />

    <!-- 플로팅 카드 -->
    <div class="card">

      <!-- 로고 -->
      <div class="logo-wrap">
        <div class="logo-icon">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none"
               stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
          </svg>
          <span class="logo-badge" />
        </div>
        <div class="logo-text">
          <h1>학생부 작성 프로그램</h1>
          <p>생활기록부 작성 및 관리 도구</p>
        </div>
      </div>

      <!-- 구분선 -->
      <div class="divider">
        <div class="divider-line" />
        <span class="divider-dot" />
        <div class="divider-line" />
      </div>

      <!-- 버튼 -->
      <div class="actions">
        <button class="btn-primary" @click="handleNew">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 4v16m8-8H4"/>
          </svg>
          새 학생부 만들기
          <svg class="arrow" width="14" height="14" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 5l7 7-7 7"/>
          </svg>
        </button>

        <button class="btn-secondary" @click="handleOpen">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"/>
          </svg>
          기존 파일 열기
          <svg class="arrow" width="14" height="14" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 5l7 7-7 7"/>
          </svg>
        </button>

        <button class="btn-ghost" @click="showGuide = true">
          사용법 안내
        </button>
      </div>

      <!-- 에러 -->
      <transition name="err">
        <div v-if="error" class="error-box">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
               style="flex-shrink:0; margin-top:1px;">
            <path d="M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
          </svg>
          {{ error }}
        </div>
      </transition>

      <p class="version">v0.1.0</p>
    </div>
  </div>

  <!-- 사용법 모달 -->
  <transition name="modal">
    <div v-if="showGuide" class="overlay" @click.self="showGuide = false">
      <div class="modal">
        <div class="modal-header">
          <div>
            <h2>사용법 안내</h2>
            <p>처음 사용하시는 분을 위한 워크플로우</p>
          </div>
          <button class="close-btn" @click="showGuide = false">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
                 stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M6 18L18 6M6 6l12 12"/>
            </svg>
          </button>
        </div>
        <div class="guide-grid">
          <div v-for="step in guideSteps" :key="step.num" class="guide-item">
            <span class="guide-num" :style="{ color: step.accent }">{{ step.num }}</span>
            <p class="guide-title">{{ step.title }}</p>
            <p class="guide-desc">{{ step.desc }}</p>
          </div>
        </div>
      </div>
    </div>
  </transition>
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
  max-width: 360px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  padding: 36px 32px 28px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255,255,255,0.03);
}

/* ── 로고 ── */
.logo-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  margin-bottom: 28px;
}

.logo-icon {
  position: relative;
  width: 56px;
  height: 56px;
  border-radius: 16px;
  background: linear-gradient(135deg, #3b5bdb, #4c6ef5);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 0 1px rgba(251,191,36,0.2), 0 8px 32px rgba(59,91,219,0.35);
}

.logo-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background-color: #fbbf24;
  border: 2px solid #0e1220;
  box-shadow: 0 0 8px rgba(251,191,36,0.5);
}

.logo-text {
  text-align: center;
}

.logo-text h1 {
  font-size: 17px;
  font-weight: 700;
  color: #e2e8f0;
  letter-spacing: -0.02em;
  margin: 0;
}

.logo-text p {
  font-size: 12px;
  color: #3d5580;
  margin: 4px 0 0;
}

/* ── 구분선 ── */
.divider {
  display: flex;
  align-items: center;
  margin-bottom: 22px;
}

.divider-line {
  flex: 1;
  height: 1px;
  background-color: #1a2035;
}

.divider-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background-color: #fbbf24;
  opacity: 0.5;
  margin: 0 10px;
}

/* ── 버튼 ── */
.actions {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.btn-primary,
.btn-secondary {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 12px 16px;
  border-radius: 12px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border: none;
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
  border: 1px solid #1a2035;
  color: #7ba3d4;
}

.btn-secondary:hover {
  background-color: #1a2640;
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

.btn-ghost {
  width: 100%;
  padding: 8px;
  background: none;
  border: none;
  font-size: 12px;
  color: #3d5580;
  cursor: pointer;
  margin-top: 2px;
  transition: color 0.15s;
}

.btn-ghost:hover {
  color: #7ba3d4;
}

/* ── 에러 ── */
.error-box {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-top: 14px;
  padding: 12px 14px;
  border-radius: 10px;
  background-color: #2a1020;
  border: 1px solid #4a1a28;
  font-size: 12px;
  color: #fca5a5;
  line-height: 1.5;
}

.err-enter-from, .err-leave-to { opacity: 0; transform: translateY(4px); }
.err-enter-active, .err-leave-active { transition: all 0.2s; }

/* ── 버전 ── */
.version {
  margin-top: 20px;
  text-align: center;
  font-size: 10px;
  color: #1e2d45;
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
  max-width: 480px;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  padding: 28px;
  box-shadow: 0 24px 80px rgba(0,0,0,0.7);
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 20px;
}

.modal-header h2 {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
  margin: 0;
}

.modal-header p {
  font-size: 11px;
  color: #3d5580;
  margin: 4px 0 0;
}

.close-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: #3d5580;
  padding: 6px;
  border-radius: 8px;
  display: flex;
  transition: background-color 0.15s, color 0.15s;
}

.close-btn:hover {
  background-color: #1a2035;
  color: #7ba3d4;
}

.guide-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.guide-item {
  background-color: #0b1020;
  border: 1px solid #1a2035;
  border-radius: 12px;
  padding: 16px;
}

.guide-num {
  display: block;
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 8px;
}

.guide-title {
  font-size: 12px;
  font-weight: 600;
  color: #c8d8f0;
  margin: 0 0 6px;
}

.guide-desc {
  font-size: 11px;
  color: #3d5580;
  line-height: 1.6;
  margin: 0;
}

.modal-enter-from, .modal-leave-to { opacity: 0; }
.modal-enter-active, .modal-leave-active { transition: opacity 0.2s; }
</style>
