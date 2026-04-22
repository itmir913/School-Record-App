<script setup>
import {BookOpen, ChevronRight, Layers, PenLine, Upload, Users} from 'lucide-vue-next'

const emit = defineEmits(['navigate'])

const steps = [
  {
    num: 1,
    icon: Users,
    title: '학생(Students) 등록',
    desc: '학년 · 반 · 번호 · 이름을 등록합니다. 엑셀 파일로 명렬표를 일괄 불러올 수 있습니다.',
    section: 'student',
    color: '#3b82f6',
    bg: 'rgba(59,130,246,0.07)',
    border: 'rgba(59,130,246,0.25)',
  },
  {
    num: 2,
    icon: Layers,
    title: '영역(Area) 구성',
    desc: '자율활동 · 진로활동 · 동아리활동 등 생기부 영역을 먼저 만들고 바이트 제한을 설정합니다.',
    section: 'area',
    color: '#a855f7',
    bg: 'rgba(168,85,247,0.07)',
    border: 'rgba(168,85,247,0.25)',
  },
  {
    num: 3,
    icon: BookOpen,
    title: '활동(Activity) 생성',
    desc: '생기부 각 영역 안에 들어갈 세부 활동을 만들고 해당 영역과 연결합니다.',
    section: 'activity',
    color: '#818cf8',
    bg: 'rgba(129,140,248,0.07)',
    border: 'rgba(129,140,248,0.25)',
  },
  {
    num: 4,
    icon: PenLine,
    title: '생기부 작성',
    desc: '학생별 · 활동별 생기부 문장을 셀 단위로 입력합니다. 바이트 제한을 자동으로 표시합니다.',
    section: 'record',
    color: '#f59e0b',
    bg: 'rgba(245,158,11,0.07)',
    border: 'rgba(245,158,11,0.25)',
  },
  {
    num: 5,
    icon: Upload,
    title: '엑셀로 내보내기',
    desc: '완성된 생기부 문장을 엑셀 파일로 저장합니다.',
    section: 'export',
    color: '#10b981',
    bg: 'rgba(16,185,129,0.07)',
    border: 'rgba(16,185,129,0.25)',
  },
]

const exampleActivities = [
  {name: '전공탐색 퀴즈 대회', desc: '학생 주도 참여'},
  {name: '전공 도서 독후감', desc: '독서 기록 활동'},
  {name: '진로 상담 프로그램', desc: '개별 상담 참여'},
  {name: '직업인 특강 청취', desc: '외부 강사 연계'},
]

const homeroomAreas = [
  {name: '자율활동', activities: ['체육대회', '현장체험학습', '학급자치 활동', '1학기 독서 토론의 날']},
  {name: '동아리활동', activities: ['○○ 동아리 활동 내용', '자율동아리 참여', '동아리 발표회']},
  {name: '진로활동', activities: ['진로상담 프로그램', '직업인 특강', '전공탐색 캠프']},
]

const subjectActivities = [
  {name: '수행평가 1', desc: '탐구 보고서 작성'},
  {name: '수행평가 2', desc: '수학 프로젝트 발표'},
  {name: '수업 태도 및 참여도', desc: '모둠 토론 · 질문 참여'},
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
        <h1 class="hero-title">5단계로 완성하는<br>체계적인 생기부 작성</h1>
        <p class="hero-sub">
          학생 명렬표 등록부터 영역 구성, 활동 생성, 생기부 작성, 내보내기까지<br>
          아래 순서대로 진행하면 학생별 학교생활기록부 문장을 손쉽게 완성할 수 있습니다.
        </p>
      </div>

      <!-- 워크플로 스텝 -->
      <div class="steps">
        <div
            v-for="step in steps"
            :key="step.num"
            class="step-card"
            :class="{ 'step-card--wide': step.num === 5 }"
            :style="{ '--c': step.color, '--bg': step.bg, '--bd': step.border }"
            @click="emit('navigate', step.section)"
        >
          <div class="step-top">
            <div class="step-num">{{ step.num }}</div>
            <component :is="step.icon" :size="28" class="step-icon" />
            <span class="step-name">{{ step.title }}</span>
          </div>
          <div class="step-desc">{{ step.desc }}</div>
          <button class="step-btn" @click.stop="emit('navigate', step.section)">
            이동하기
            <ChevronRight :size="15"/>
          </button>
        </div>
      </div>

      <!-- 구조 설명 -->
      <div class="structure">
        <div class="structure-header">
          <h2 class="structure-title">이 프로그램은 어떻게 작동하나요?</h2>
          <p class="structure-sub">
            생기부의 각 항목을 <strong>영역(Area)</strong>이라고 부릅니다.
            예를 들어 <strong>진로활동</strong>은 하나의 영역입니다.<br>
            그 안에 학생이 실제로 참여한 개별 활동들, 즉 <strong>활동(Activity)</strong> 여러 개를 담아 하나의 영역을 완성합니다.
          </p>
        </div>

        <!-- Area 박스 다이어그램 -->
        <div class="area-box">
          <div class="area-box-header">
            <span class="area-tag">Area</span>
            <span class="area-box-name">진로활동</span>
            <span class="area-box-limit">최대 1,500 byte</span>
          </div>

          <div class="area-box-desc">
            아래 Activity들의 기록이 합쳐져 이 영역 하나를 구성합니다.
          </div>

          <div class="activities-grid">
            <div
                v-for="act in exampleActivities"
                :key="act.name"
                class="activity-card"
            >
              <div class="activity-dot"></div>
              <div class="activity-info">
                <div class="activity-name">{{ act.name }}</div>
                <div class="activity-sub">{{ act.desc }}</div>
              </div>
            </div>

            <!-- 추가 가능 암시 카드 -->
            <div class="activity-card activity-card--more">
              <div class="activity-more-icon">＋</div>
              <div class="activity-info">
                <div class="activity-name" style="color:#5a7090;">활동 더 추가 가능</div>
                <div class="activity-sub">원하는 만큼</div>
              </div>
            </div>
          </div>

          <div class="area-box-footer">
            각 Activity마다 학생별로 기록을 작성하면, 합산 문장이 <strong>진로활동</strong> 항목으로 완성됩니다.
          </div>
        </div>
      </div>

      <!-- 담임교사 활용 가이드 -->
      <div class="usecase">
        <div class="usecase-header">
          <span class="role-badge role-badge--homeroom">담임교사</span>
          <h2 class="usecase-title">담임교사로 활용하기</h2>
        </div>
        <p class="usecase-desc">
          담임교사는 <strong>자율활동 · 동아리활동 · 진로활동</strong> 3개 영역(Area)의 생기부를 작성합니다.
          각 영역은 학생이 참여한 개별 행사 혹은 프로그램, 즉 <strong>활동(Activity)</strong>의 기록을
          하나로 합쳐서 완성됩니다. 지도교사들이 엑셀로 제공한 기재 문장을
          <strong>가져오기(Import)</strong> 탭으로 불러오면, 활동별 문장이 자동으로 채워지고
          합산 바이트를 실시간으로 확인하며 영역 제한을 맞출 수 있습니다.
        </p>

        <div class="mapping-bar">
          <div class="mapping-item">
            <span class="mapping-key mapping-key--area">영역 (Area)</span>
            <span class="mapping-arrow">→</span>
            <span class="mapping-val">자율활동 · 동아리활동 · 진로활동</span>
          </div>
          <div class="mapping-divider"></div>
          <div class="mapping-item">
            <span class="mapping-key mapping-key--act">활동 (Activity)</span>
            <span class="mapping-arrow">→</span>
            <span class="mapping-val">체육대회, 현장체험학습, 진로상담 프로그램 …</span>
          </div>
        </div>

        <div class="homeroom-grid">
          <div v-for="area in homeroomAreas" :key="area.name" class="uc-area-box">
            <div class="uc-area-header">
              <span class="uc-area-tag">Area</span>
              <span class="uc-area-name">{{ area.name }}</span>
            </div>
            <div class="uc-acts">
              <div v-for="act in area.activities" :key="act" class="uc-act-row">
                <span class="uc-act-dot"></span>
                <span class="uc-act-label">{{ act }}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="usecase-tip">
          지도교사에게 기재 문장을 <strong>엑셀 파일</strong>로 받아 <strong>가져오기(Import)</strong> 탭에서
          불러오면 활동별 문장이 자동으로 채워집니다. 영역 바이트를 초과하는 항목은 즉시 표시되므로
          빠르게 파악하고 조절할 수 있습니다.
        </div>
      </div>

      <!-- 교과교사 활용 가이드 -->
      <div class="usecase">
        <div class="usecase-header">
          <span class="role-badge role-badge--subject">교과교사</span>
          <h2 class="usecase-title">교과교사로 활용하기</h2>
        </div>
        <p class="usecase-desc">
          교과교사는 담당 과목의 <strong>세부능력 및 특기사항(세특)</strong>을 작성합니다.
          세특 하나가 <strong>영역(Area)</strong>이 되고, 수행평가 · 수업태도처럼 세특을 구성하는
          개별 항목이 <strong>활동(Activity)</strong>이 됩니다. 항목마다 문장을 별도로 작성한 뒤
          합산 바이트를 확인하며 완성도 높은 세특을 만들 수 있습니다.
        </p>

        <div class="mapping-bar">
          <div class="mapping-item">
            <span class="mapping-key mapping-key--area">영역 (Area)</span>
            <span class="mapping-arrow">→</span>
            <span class="mapping-val">세부능력 및 특기사항 (과목명)</span>
          </div>
          <div class="mapping-divider"></div>
          <div class="mapping-item">
            <span class="mapping-key mapping-key--act">활동 (Activity)</span>
            <span class="mapping-arrow">→</span>
            <span class="mapping-val">수행평가 1, 수행평가 2, 수업 태도 …</span>
          </div>
        </div>

        <div class="uc-area-box uc-area-box--subject">
          <div class="uc-area-header">
            <span class="uc-area-tag uc-area-tag--subject">Area</span>
            <span class="uc-area-name">수학 세부능력 및 특기사항</span>
            <span class="uc-area-limit">최대 1,500 byte</span>
          </div>
          <div class="uc-acts">
            <div v-for="act in subjectActivities" :key="act.name" class="uc-act-row">
              <span class="uc-act-dot uc-act-dot--subject"></span>
              <div>
                <div class="uc-act-label">{{ act.name }}</div>
                <div class="uc-act-sub">{{ act.desc }}</div>
              </div>
            </div>
          </div>
          <div class="uc-area-footer">
            세 활동의 문장을 합치면 <strong>수학 세특 영역</strong>이 완성됩니다.
          </div>
        </div>

        <div class="usecase-tip usecase-tip--subject">
          과목별로 프로젝트를 따로 만들거나, 한 프로젝트 안에 여러 과목을 영역으로 묶어 관리할 수 있습니다.
          학생 수가 많을수록 격자형 입력 화면이 큰 도움이 됩니다.
        </div>
      </div>

    </div>
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
  padding: 44px 44px 60px;
  display: flex;
  flex-direction: column;
  gap: 52px;
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
  font-size: 16px;
  font-weight: 500;
  color: #93b8d8;
}

.eyebrow-badge {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.06em;
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.12);
  border: 1px solid rgba(251, 191, 36, 0.28);
  border-radius: 5px;
  padding: 3px 10px;
}

.hero-title {
  font-size: 46px;
  font-weight: 800;
  color: #eef2f8;
  margin: 0;
  line-height: 1.25;
  letter-spacing: -0.025em;
}

.hero-sub {
  font-size: 17px;
  color: #93b8d8;
  margin: 0;
  line-height: 1.85;
}

/* ── 워크플로 스텝 ───────────────────────────────────────────── */
.steps {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

.step-card {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 12px;
  padding: 26px 28px;
  background: var(--bg);
  border: 1px solid var(--bd);
  border-radius: 16px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s, transform 0.15s;
}

.step-card--wide {
  grid-column: 1 / -1;
}

.step-card:hover {
  border-color: var(--c);
  box-shadow: 0 4px 24px color-mix(in srgb, var(--c) 18%, transparent);
  transform: translateY(-2px);
}

.step-top {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: nowrap;
  overflow: hidden;
}

.step-num {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--c) 14%, transparent);
  border: 2px solid color-mix(in srgb, var(--c) 35%, transparent);
  color: var(--c);
  font-size: 21px;
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

.step-name {
  font-size: 20px;
  font-weight: 700;
  color: #eef2f8;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.step-desc {
  font-size: 15px;
  color: #8bb2cc;
  line-height: 1.7;
  flex: 1;
}

.step-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 9px 18px;
  border-radius: 9px;
  background: color-mix(in srgb, var(--c) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--c) 30%, transparent);
  color: var(--c);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  align-self: flex-start;
  margin-top: 4px;
  transition: background 0.15s, border-color 0.15s;
}

.step-btn:hover {
  background: color-mix(in srgb, var(--c) 22%, transparent);
  border-color: var(--c);
}

/* ── 구조 설명 ───────────────────────────────────────────────── */
.structure {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.structure-header {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.structure-title {
  font-size: 26px;
  font-weight: 700;
  color: #eef2f8;
  margin: 0;
}

.structure-sub {
  font-size: 16px;
  color: #93b8d8;
  line-height: 1.85;
  margin: 0;
}

.structure-sub strong {
  color: #c8d8f0;
  font-weight: 700;
}

.structure-sub em {
  font-style: normal;
  color: #c8d8f0;
}

/* ── Area 박스 다이어그램 ────────────────────────────────────── */
.area-box {
  border: 2px solid rgba(168, 85, 247, 0.35);
  border-radius: 20px;
  background: rgba(168, 85, 247, 0.04);
  padding: 28px 32px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.area-box-header {
  display: flex;
  align-items: center;
  gap: 14px;
}

.area-tag {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.06em;
  color: #a855f7;
  background: rgba(168, 85, 247, 0.16);
  border: 1px solid rgba(168, 85, 247, 0.35);
  border-radius: 6px;
  padding: 4px 12px;
}

.area-box-name {
  font-size: 26px;
  font-weight: 800;
  color: #eef2f8;
}

.area-box-limit {
  font-size: 14px;
  color: #718fad;
  margin-left: auto;
  border: 1px solid #30395c;
  border-radius: 6px;
  padding: 4px 12px;
}

.area-box-desc {
  font-size: 15px;
  color: #6a8aaa;
  padding-bottom: 4px;
  border-bottom: 1px solid #1a2035;
}

.activities-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}

.activity-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 18px;
  background: #0d1220;
  border: 1px solid rgba(129, 140, 248, 0.25);
  border-radius: 12px;
  transition: border-color 0.15s;
}

.activity-card:hover {
  border-color: rgba(129, 140, 248, 0.5);
}

.activity-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #818cf8;
  flex-shrink: 0;
  opacity: 0.75;
}

.activity-card--more {
  border-style: dashed;
  border-color: #1e2a45;
  background: transparent;
}

.activity-more-icon {
  width: 10px;
  text-align: center;
  font-size: 16px;
  color: #3a4a6b;
  flex-shrink: 0;
}

.activity-info {
  min-width: 0;
}

.activity-name {
  font-size: 16px;
  font-weight: 600;
  color: #d0e0f0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.activity-sub {
  font-size: 13px;
  color: #5a7090;
  margin-top: 3px;
}

.area-box-footer {
  font-size: 15px;
  color: #6a8aaa;
  padding-top: 4px;
  border-top: 1px solid #1a2035;
  line-height: 1.75;
}

.area-box-footer strong {
  color: #a880f0;
  font-weight: 600;
}

/* ── 활용 가이드 (담임교사 / 교과교사) ──────────────────────── */
.usecase {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.usecase-header {
  display: flex;
  align-items: center;
  gap: 14px;
}

.role-badge {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.05em;
  border-radius: 6px;
  padding: 4px 13px;
  flex-shrink: 0;
}

.role-badge--homeroom {
  color: #10b981;
  background: rgba(16, 185, 129, 0.12);
  border: 1px solid rgba(16, 185, 129, 0.3);
}

.role-badge--subject {
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.12);
  border: 1px solid rgba(245, 158, 11, 0.3);
}

.usecase-title {
  font-size: 24px;
  font-weight: 700;
  color: #eef2f8;
  margin: 0;
}

.usecase-desc {
  font-size: 16px;
  color: #93b8d8;
  line-height: 1.85;
  margin: 0;
}

.usecase-desc strong {
  color: #c8d8f0;
  font-weight: 700;
}

/* 매핑 바 */
.mapping-bar {
  display: flex;
  align-items: center;
  background: #0a0f1e;
  border: 1px solid #1e2a45;
  border-radius: 12px;
  padding: 18px 24px;
  flex-wrap: wrap;
  gap: 12px;
}

.mapping-item {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 200px;
}

.mapping-divider {
  width: 1px;
  height: 32px;
  background: #1e2a45;
  flex-shrink: 0;
}

.mapping-key {
  font-size: 13px;
  font-weight: 700;
  border-radius: 6px;
  padding: 5px 12px;
  white-space: nowrap;
}

.mapping-key--area {
  color: #a855f7;
  background: rgba(168, 85, 247, 0.12);
  border: 1px solid rgba(168, 85, 247, 0.3);
}

.mapping-key--act {
  color: #818cf8;
  background: rgba(129, 140, 248, 0.12);
  border: 1px solid rgba(129, 140, 248, 0.3);
}

.mapping-arrow {
  font-size: 15px;
  color: #3a4a6b;
  flex-shrink: 0;
}

.mapping-val {
  font-size: 15px;
  color: #8bb2cc;
}

/* 담임교사 3단 그리드 */
.homeroom-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 14px;
}

/* 공통 uc-area 박스 */
.uc-area-box {
  border: 2px solid rgba(168, 85, 247, 0.3);
  border-radius: 16px;
  background: rgba(168, 85, 247, 0.04);
  padding: 22px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.uc-area-box--subject {
  border-color: rgba(245, 158, 11, 0.3);
  background: rgba(245, 158, 11, 0.03);
}

.uc-area-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.uc-area-tag {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.06em;
  color: #a855f7;
  background: rgba(168, 85, 247, 0.16);
  border: 1px solid rgba(168, 85, 247, 0.35);
  border-radius: 5px;
  padding: 3px 9px;
  flex-shrink: 0;
}

.uc-area-tag--subject {
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.14);
  border-color: rgba(245, 158, 11, 0.35);
}

.uc-area-name {
  font-size: 17px;
  font-weight: 700;
  color: #eef2f8;
}

.uc-area-limit {
  font-size: 13px;
  color: #718fad;
  margin-left: auto;
  border: 1px solid #30395c;
  border-radius: 6px;
  padding: 3px 10px;
  white-space: nowrap;
}

.uc-acts {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.uc-act-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.uc-act-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #818cf8;
  flex-shrink: 0;
  margin-top: 5px;
  opacity: 0.7;
}

.uc-act-dot--subject {
  background: #f59e0b;
}

.uc-act-label {
  font-size: 15px;
  font-weight: 500;
  color: #c8d8f0;
  line-height: 1.5;
}

.uc-act-sub {
  font-size: 13px;
  color: #5a7090;
  margin-top: 2px;
}

.uc-area-footer {
  font-size: 14px;
  color: #6a8aaa;
  padding-top: 12px;
  border-top: 1px solid #1a2035;
  line-height: 1.75;
}

.uc-area-footer strong {
  color: #f59e0b;
  font-weight: 600;
}

/* 팁 박스 */
.usecase-tip {
  font-size: 15px;
  color: #7aa5c8;
  background: rgba(16, 185, 129, 0.05);
  border: 1px solid rgba(16, 185, 129, 0.18);
  border-left: 3px solid #10b981;
  border-radius: 8px;
  padding: 14px 18px;
  line-height: 1.75;
}

.usecase-tip strong {
  color: #a7f0d6;
  font-weight: 600;
}

.usecase-tip--subject {
  background: rgba(245, 158, 11, 0.05);
  border-color: rgba(245, 158, 11, 0.18);
  border-left-color: #f59e0b;
}

.usecase-tip--subject strong {
  color: #fcd37a;
}
</style>
