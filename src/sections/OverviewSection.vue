<script setup>
import {BookOpen, ChevronRight, Layers, PenLine, Upload, Users} from 'lucide-vue-next'

const emit = defineEmits(['select'])

const steps = [
  {
    num: 1,
    icon: Users,
    title: '학생 등록',
    desc: '학년·반·번호·이름을 등록합니다. 엑셀 파일로 명렬표를 일괄 불러올 수 있습니다.',
    section: 'student',
    color: '#3b82f6',
    bg: 'rgba(59,130,246,0.07)',
    border: 'rgba(59,130,246,0.25)',
  },
  {
    num: 2,
    icon: BookOpen,
    title: 'Activity 생성',
    desc: '수업·프로젝트·동아리 등 생기부에 들어갈 각 활동 단위를 만듭니다.',
    section: 'activity',
    color: '#818cf8',
    bg: 'rgba(129,140,248,0.07)',
    border: 'rgba(129,140,248,0.25)',
  },
  {
    num: 3,
    icon: Layers,
    title: 'Area 구성',
    desc: '여러 Activity를 묶어 진로활동·자율활동·동아리 등 생기부 영역을 완성합니다.',
    section: 'area',
    color: '#a855f7',
    bg: 'rgba(168,85,247,0.07)',
    border: 'rgba(168,85,247,0.25)',
  },
  {
    num: 4,
    icon: PenLine,
    title: '기록 작성',
    desc: '학생별·활동별 생기부 문장을 셀 단위로 입력합니다. 바이트 제한을 자동으로 표시합니다.',
    section: 'record',
    color: '#f59e0b',
    bg: 'rgba(245,158,11,0.07)',
    border: 'rgba(245,158,11,0.25)',
  },
  {
    num: 5,
    icon: Upload,
    title: '내보내기',
    desc: '완성된 생기부 문장을 엑셀 파일로 추출합니다. A·B·C 세 가지 형식을 지원합니다.',
    section: 'export',
    color: '#10b981',
    bg: 'rgba(16,185,129,0.07)',
    border: 'rgba(16,185,129,0.25)',
  },
]
</script>

<template>
  <div class="section">

    <div class="section-body">

      <!-- 히어로 -->
      <div class="hero">
        <div class="hero-eyebrow">
          <span class="eyebrow-badge">생기부</span>
          학교생활기록부 작성 도우미
        </div>
        <h1 class="hero-title">5단계로 완성하는<br>체계적인 생기부 관리</h1>
        <p class="hero-sub">
          학생 명렬표 등록부터 Activity 조합, 영역 구성, 기록 작성, 내보내기까지<br>
          아래 순서대로 진행하면 학생별 생활기록부 문장을 완성할 수 있습니다.
        </p>
      </div>

      <!-- 워크플로 스텝 -->
      <div class="steps">
        <template v-for="(step, idx) in steps" :key="step.num">

          <div
              class="step-card"
              :style="{
              '--c': step.color,
              '--bg': step.bg,
              '--bd': step.border,
            }"
              @click="emit('select', step.section)"
          >
            <div class="step-left">
              <div class="step-num">{{ step.num }}</div>
              <component :is="step.icon" :size="28" class="step-icon"/>
            </div>
            <div class="step-middle">
              <div class="step-name">{{ step.title }}</div>
              <div class="step-desc">{{ step.desc }}</div>
            </div>
            <button class="step-btn" @click.stop="emit('select', step.section)">
              이동하기
              <ChevronRight :size="16"/>
            </button>
          </div>

          <div v-if="idx < steps.length - 1" class="connector">
            <div class="connector-line"/>
            <div class="connector-arrow">↓</div>
            <div class="connector-line"/>
          </div>

        </template>
      </div>

      <!-- 도메인 구조 -->
      <div class="domain">
        <div class="domain-title-row">
          <span class="domain-eyebrow">핵심 구조 이해</span>
          <span class="domain-hint">하나의 Area 안에 여러 Activity가 포함되며, 각 Activity마다 학생별 기록을 작성합니다.</span>
        </div>

        <div class="domain-diagram">

          <div class="dom-block dom-block--area">
            <div class="dom-tag"
                 style="color:#a855f7; background:rgba(168,85,247,0.14); border-color:rgba(168,85,247,0.3);">Area
            </div>
            <div class="dom-block-name">진로활동</div>
            <div class="dom-block-sub">생기부 대분류 영역</div>
          </div>

          <div class="dom-rel">
            <div class="dom-rel-line"/>
            <span class="dom-rel-label">1 : N</span>
            <div class="dom-rel-line"/>
          </div>

          <div class="dom-activities-col">
            <div class="dom-tag"
                 style="color:#818cf8; background:rgba(129,140,248,0.14); border-color:rgba(129,140,248,0.3); margin-bottom:12px;">
              Activities
            </div>
            <div class="dom-chip">AI 탐구 프로젝트</div>
            <div class="dom-chip">진로 독서 발표</div>
            <div class="dom-chip">직업인 특강 참여</div>
          </div>

          <div class="dom-rel">
            <div class="dom-rel-line"/>
            <span class="dom-rel-label">조합</span>
            <div class="dom-rel-line"/>
          </div>

          <div class="dom-block dom-block--result">
            <div class="dom-tag"
                 style="color:#10b981; background:rgba(16,185,129,0.14); border-color:rgba(16,185,129,0.3);">결과
            </div>
            <div class="dom-block-name" style="font-size:15px; line-height:1.65;">학생별 생기부<br>문장 완성</div>
            <div class="dom-block-sub">바이트 제한 자동 관리</div>
          </div>

        </div>
      </div>

    </div>
  </div>
</template>

<style scoped>
/* ── 공통 섹션 레이아웃 ─────────────────────────────────────── */
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
  padding: 40px 40px 56px;
  display: flex;
  flex-direction: column;
  gap: 48px;
}

/* ── 히어로 ─────────────────────────────────────────────────── */
.hero {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.hero-eyebrow {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
  font-weight: 500;
  color: #7ba3d4;
}

.eyebrow-badge {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.12);
  border: 1px solid rgba(251, 191, 36, 0.28);
  border-radius: 5px;
  padding: 2px 8px;
}

.hero-title {
  font-size: 36px;
  font-weight: 800;
  color: #e2e8f0;
  margin: 0;
  line-height: 1.3;
  letter-spacing: -0.02em;
}

.hero-sub {
  font-size: 15px;
  color: #7ba3d4;
  margin: 0;
  line-height: 1.75;
}

/* ── 워크플로 스텝 ───────────────────────────────────────────── */
.steps {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.step-card {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 24px 28px;
  background: var(--bg);
  border: 1px solid var(--bd);
  border-radius: 16px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s, transform 0.15s;
}

.step-card:hover {
  border-color: var(--c);
  box-shadow: 0 4px 24px color-mix(in srgb, var(--c) 18%, transparent);
  transform: translateX(4px);
}

.step-left {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-shrink: 0;
}

.step-num {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--c) 14%, transparent);
  border: 2px solid color-mix(in srgb, var(--c) 35%, transparent);
  color: var(--c);
  font-size: 20px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.step-icon {
  color: var(--c);
  opacity: 0.85;
}

.step-middle {
  flex: 1;
  min-width: 0;
}

.step-name {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin-bottom: 6px;
}

.step-desc {
  font-size: 14px;
  color: #7ba3d4;
  line-height: 1.6;
}

.step-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 9px 18px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--c) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--c) 30%, transparent);
  color: var(--c);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.15s, border-color 0.15s;
  white-space: nowrap;
}

.step-btn:hover {
  background: color-mix(in srgb, var(--c) 22%, transparent);
  border-color: var(--c);
}

.connector {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding-left: 52px;
  gap: 0;
}

.connector-line {
  width: 1px;
  height: 10px;
  background: #1e2a45;
  margin-left: 25px;
}

.connector-arrow {
  font-size: 14px;
  color: #2a3a5a;
  margin-left: 17px;
  line-height: 1;
}

/* ── 도메인 구조 ─────────────────────────────────────────────── */
.domain {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.domain-title-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.domain-eyebrow {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
  color: #4a6080;
  flex-shrink: 0;
}

.domain-hint {
  font-size: 13px;
  color: #4a6080;
}

.domain-diagram {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 28px 32px;
  background: #0d1220;
  border: 1px solid #1a2035;
  border-radius: 16px;
  flex-wrap: wrap;
}

.dom-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 20px 24px;
  border-radius: 12px;
  border: 1px solid #1e2a45;
  background: #080b14;
  flex-shrink: 0;
  min-width: 150px;
}

.dom-tag {
  display: inline-flex;
  align-items: center;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.06em;
  border-radius: 999px;
  border: 1px solid;
  padding: 2px 10px;
  width: fit-content;
}

.dom-block-name {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
}

.dom-block-sub {
  font-size: 12px;
  color: #4a6080;
}

.dom-rel {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.dom-rel-line {
  width: 24px;
  height: 1px;
  background: #1e2a45;
}

.dom-rel-label {
  font-size: 11px;
  color: #3a4a6b;
  font-weight: 600;
  white-space: nowrap;
}

.dom-activities-col {
  display: flex;
  flex-direction: column;
  gap: 7px;
  flex-shrink: 0;
}

.dom-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #c8d8f0;
  background: rgba(129, 140, 248, 0.08);
  border: 1px solid rgba(129, 140, 248, 0.22);
  border-radius: 8px;
  padding: 7px 14px;
  white-space: nowrap;
}
</style>
