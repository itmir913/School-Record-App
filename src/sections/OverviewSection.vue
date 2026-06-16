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
  <div class="flex flex-col h-full overflow-hidden box-border">
    <div class="flex-1 overflow-y-auto px-11 pt-11 pb-[60px] flex flex-col gap-[52px]">

      <!-- 히어로 -->
      <div class="flex flex-col gap-4">
        <div class="flex items-center gap-2.5 text-base font-medium text-[#93b8d8]">
          <span class="text-xs font-bold tracking-[0.06em] text-amber bg-amber/[0.12] border border-amber/[0.28] rounded-[5px] px-2.5 py-[3px]">생기부</span>
          학교생활기록부 작성 도우미
        </div>
        <h1 class="text-[46px] font-extrabold text-[#eef2f8] m-0 leading-[1.25] tracking-[-0.025em]">5단계로 완성하는<br>체계적인 생기부 작성</h1>
        <p class="text-[17px] text-[#93b8d8] m-0 leading-[1.85]">
          학생 명렬표 등록부터 영역 구성, 활동 생성, 생기부 작성, 내보내기까지<br>
          아래 순서대로 진행하면 학생별 학교생활기록부 문장을 손쉽게 완성할 수 있습니다.
        </p>
      </div>

      <!-- 워크플로 스텝 -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-3.5">
        <div
            v-for="step in steps"
            :key="step.num"
            :class="['flex flex-col items-stretch gap-3 py-[26px] px-7 bg-[var(--bg)] border border-[var(--bd)] rounded-2xl cursor-pointer transition-[border-color,box-shadow,transform] hover:border-[var(--c)] hover:shadow-[0_4px_24px_color-mix(in_srgb,var(--c)_18%,transparent)] hover:-translate-y-0.5', step.num === 5 && 'lg:col-span-2']"
            :style="{ '--c': step.color, '--bg': step.bg, '--bd': step.border }"
            @click="emit('navigate', step.section)"
        >
          <div class="flex items-center gap-3">
            <div class="w-[52px] h-[52px] rounded-full bg-[color-mix(in_srgb,var(--c)_14%,transparent)] border-2 border-[color-mix(in_srgb,var(--c)_35%,transparent)] text-[var(--c)] text-[21px] font-extrabold flex items-center justify-center flex-shrink-0">
              {{ step.num }}
            </div>
            <component :is="step.icon" :size="28" class="text-[var(--c)] opacity-[0.85]"/>
            <span class="text-xl font-bold text-[#eef2f8] flex-1">{{ step.title }}</span>
          </div>
          <div class="text-[15px] text-[#8bb2cc] leading-[1.7] flex-1">{{ step.desc }}</div>
          <button
              class="flex items-center gap-[5px] px-[18px] py-[9px] rounded-[9px] bg-[color-mix(in_srgb,var(--c)_12%,transparent)] border border-[color-mix(in_srgb,var(--c)_30%,transparent)] text-[var(--c)] text-sm font-semibold cursor-pointer whitespace-nowrap self-start mt-1 transition-[background,border-color] hover:bg-[color-mix(in_srgb,var(--c)_22%,transparent)] hover:border-[var(--c)]"
              @click.stop="emit('navigate', step.section)"
          >
            이동하기
            <ChevronRight :size="15"/>
          </button>
        </div>
      </div>

      <!-- 구조 설명 -->
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-3">
          <h2 class="text-[26px] font-bold text-[#eef2f8] m-0">이 프로그램은 어떻게 작동하나요?</h2>
          <p class="text-base text-[#93b8d8] leading-[1.85] m-0">
            생기부의 각 항목을 <strong class="text-[#c8d8f0] font-bold">영역(Area)</strong>이라고 부릅니다.
            예를 들어 <strong class="text-[#c8d8f0] font-bold">진로활동</strong>은 하나의 영역입니다.<br>
            그 안에 학생이 실제로 참여한 개별 활동들, 즉 <strong class="text-[#c8d8f0] font-bold">활동(Activity)</strong> 여러 개를 담아 하나의 영역을 완성합니다.
          </p>
        </div>

        <!-- Area 박스 다이어그램 -->
        <div class="border-2 border-violet/35 rounded-modal bg-violet/[0.04] px-8 py-7 flex flex-col gap-5">
          <div class="flex items-center gap-3.5">
            <span class="text-sm font-bold tracking-[0.06em] text-violet bg-violet/[0.16] border border-violet/35 rounded-md px-3 py-1">Area</span>
            <span class="text-[26px] font-extrabold text-[#eef2f8]">진로활동</span>
            <span class="text-sm text-[#718fad] ml-auto border border-[#30395c] rounded-md px-3 py-1">최대 1,500 byte</span>
          </div>

          <div class="text-[15px] text-[#6a8aaa] pb-1 border-b border-line">
            아래 Activity들의 기록이 합쳐져 이 영역 하나를 구성합니다.
          </div>

          <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(200px, 1fr))">
            <div
                v-for="act in exampleActivities"
                :key="act.name"
                class="flex items-center gap-3.5 px-[18px] py-3.5 bg-[#0d1220] border border-[rgba(129,140,248,0.25)] rounded-xl transition-colors hover:border-[rgba(129,140,248,0.5)]"
            >
              <div class="w-2.5 h-2.5 rounded-full bg-[#818cf8] flex-shrink-0 opacity-75"/>
              <div class="min-w-0">
                <div class="text-base font-semibold text-[#d0e0f0] whitespace-nowrap overflow-hidden text-ellipsis">{{ act.name }}</div>
                <div class="text-sm text-[#5a7090] mt-[3px]">{{ act.desc }}</div>
              </div>
            </div>

            <!-- 추가 가능 암시 카드 -->
            <div class="flex items-center gap-3.5 px-[18px] py-3.5 bg-transparent border border-dashed border-[#1e2a45] rounded-xl">
              <div class="w-2.5 text-center text-base text-[#3a4a6b] flex-shrink-0">＋</div>
              <div class="min-w-0">
                <div class="text-base font-semibold whitespace-nowrap overflow-hidden text-ellipsis" style="color:#5a7090;">활동 더 추가 가능</div>
                <div class="text-sm text-[#5a7090] mt-[3px]">원하는 만큼</div>
              </div>
            </div>
          </div>

          <div class="text-[15px] text-[#6a8aaa] pt-1 border-t border-line leading-[1.75]">
            각 Activity마다 학생별로 기록을 작성하면, 합산 문장이 <strong class="text-[#a880f0] font-semibold">진로활동</strong> 항목으로 완성됩니다.
          </div>
        </div>
      </div>

      <!-- 담임교사 활용 가이드 -->
      <div class="flex flex-col gap-5">
        <div class="flex items-center gap-3.5">
          <span class="text-sm font-bold tracking-[0.05em] rounded-md px-[13px] py-1 flex-shrink-0 text-green bg-green/[0.12] border border-green/30">담임교사</span>
          <h2 class="text-[24px] font-bold text-[#eef2f8] m-0">담임교사로 활용하기</h2>
        </div>
        <p class="text-base text-[#93b8d8] leading-[1.85] m-0">
          담임교사는 <strong class="text-[#c8d8f0] font-bold">자율활동 · 동아리활동 · 진로활동</strong> 3개 영역(Area)의 생기부를 작성합니다.
          각 영역은 학생이 참여한 개별 행사 혹은 프로그램, 즉 <strong class="text-[#c8d8f0] font-bold">활동(Activity)</strong>의 기록을
          하나로 합쳐서 완성됩니다. 지도교사들이 엑셀로 제공한 기재 문장을
          <strong class="text-[#c8d8f0] font-bold">가져오기(Import)</strong> 탭으로 불러오면, 활동별 문장이 자동으로 채워지고
          합산 바이트를 실시간으로 확인하며 영역 제한을 맞출 수 있습니다.
        </p>

        <div class="flex items-center bg-base border border-[#1e2a45] rounded-xl px-6 py-[18px] flex-wrap gap-3">
          <div class="flex items-center gap-2.5 flex-1 min-w-[200px]">
            <span class="text-sm font-bold rounded-md px-3 py-[5px] whitespace-nowrap text-violet bg-violet/[0.12] border border-violet/30">영역 (Area)</span>
            <span class="text-[15px] text-[#3a4a6b] flex-shrink-0">→</span>
            <span class="text-[15px] text-[#8bb2cc]">자율활동 · 동아리활동 · 진로활동</span>
          </div>
          <div class="w-px h-8 bg-[#1e2a45] flex-shrink-0"/>
          <div class="flex items-center gap-2.5 flex-1 min-w-[200px]">
            <span class="text-sm font-bold rounded-md px-3 py-[5px] whitespace-nowrap text-[#818cf8] bg-[rgba(129,140,248,0.12)] border border-[rgba(129,140,248,0.3)]">활동 (Activity)</span>
            <span class="text-[15px] text-[#3a4a6b] flex-shrink-0">→</span>
            <span class="text-[15px] text-[#8bb2cc]">체육대회, 현장체험학습, 진로상담 프로그램 …</span>
          </div>
        </div>

        <div class="grid grid-cols-3 gap-3.5">
          <div v-for="area in homeroomAreas" :key="area.name"
               class="border-2 border-violet/30 rounded-2xl bg-violet/[0.04] px-6 py-[22px] flex flex-col gap-3.5">
            <div class="flex items-center gap-2.5">
              <span class="text-xs font-bold tracking-[0.06em] text-violet bg-violet/[0.16] border border-violet/35 rounded-[5px] px-[9px] py-[3px] flex-shrink-0">Area</span>
              <span class="text-[17px] font-bold text-[#eef2f8]">{{ area.name }}</span>
            </div>
            <div class="flex flex-col gap-2">
              <div v-for="act in area.activities" :key="act" class="flex items-start gap-2.5">
                <span class="w-2 h-2 rounded-full bg-[#818cf8] flex-shrink-0 mt-[5px] opacity-70"/>
                <span class="text-[15px] font-medium text-[#c8d8f0] leading-[1.5]">{{ act }}</span>
              </div>
            </div>
          </div>
        </div>

        <div
            class="text-[15px] text-[#7aa5c8] bg-green/[0.05] rounded-lg px-[18px] py-3.5 leading-[1.75]"
            style="border: 1px solid rgba(16,185,129,0.18); border-left: 3px solid #10b981;"
        >
          지도교사에게 기재 문장을 <strong class="text-[#a7f0d6] font-semibold">엑셀 파일</strong>로 받아 <strong class="text-[#a7f0d6] font-semibold">가져오기(Import)</strong> 탭에서
          불러오면 활동별 문장이 자동으로 채워집니다. 영역 바이트를 초과하는 항목은 즉시 표시되므로
          빠르게 파악하고 조절할 수 있습니다.
        </div>
      </div>

      <!-- 교과교사 활용 가이드 -->
      <div class="flex flex-col gap-5">
        <div class="flex items-center gap-3.5">
          <span class="text-sm font-bold tracking-[0.05em] rounded-md px-[13px] py-1 flex-shrink-0 text-amber bg-amber/[0.12] border border-amber/30">교과교사</span>
          <h2 class="text-[24px] font-bold text-[#eef2f8] m-0">교과교사로 활용하기</h2>
        </div>
        <p class="text-base text-[#93b8d8] leading-[1.85] m-0">
          교과교사는 담당 과목의 <strong class="text-[#c8d8f0] font-bold">세부능력 및 특기사항(세특)</strong>을 작성합니다.
          세특 하나가 <strong class="text-[#c8d8f0] font-bold">영역(Area)</strong>이 되고, 수행평가 · 수업태도처럼 세특을 구성하는
          개별 항목이 <strong class="text-[#c8d8f0] font-bold">활동(Activity)</strong>이 됩니다. 항목마다 문장을 별도로 작성한 뒤
          합산 바이트를 확인하며 완성도 높은 세특을 만들 수 있습니다.
        </p>

        <div class="flex items-center bg-base border border-[#1e2a45] rounded-xl px-6 py-[18px] flex-wrap gap-3">
          <div class="flex items-center gap-2.5 flex-1 min-w-[200px]">
            <span class="text-sm font-bold rounded-md px-3 py-[5px] whitespace-nowrap text-violet bg-violet/[0.12] border border-violet/30">영역 (Area)</span>
            <span class="text-[15px] text-[#3a4a6b] flex-shrink-0">→</span>
            <span class="text-[15px] text-[#8bb2cc]">세부능력 및 특기사항 (과목명)</span>
          </div>
          <div class="w-px h-8 bg-[#1e2a45] flex-shrink-0"/>
          <div class="flex items-center gap-2.5 flex-1 min-w-[200px]">
            <span class="text-sm font-bold rounded-md px-3 py-[5px] whitespace-nowrap text-[#818cf8] bg-[rgba(129,140,248,0.12)] border border-[rgba(129,140,248,0.3)]">활동 (Activity)</span>
            <span class="text-[15px] text-[#3a4a6b] flex-shrink-0">→</span>
            <span class="text-[15px] text-[#8bb2cc]">수행평가 1, 수행평가 2, 수업 태도 …</span>
          </div>
        </div>

        <div class="border-2 border-amber/30 rounded-2xl bg-amber/[0.03] px-6 py-[22px] flex flex-col gap-3.5">
          <div class="flex items-center gap-2.5">
            <span class="text-xs font-bold tracking-[0.06em] text-amber bg-amber/[0.14] border border-amber/35 rounded-[5px] px-[9px] py-[3px] flex-shrink-0">Area</span>
            <span class="text-[17px] font-bold text-[#eef2f8]">수학 세부능력 및 특기사항</span>
            <span class="text-sm text-[#718fad] ml-auto border border-[#30395c] rounded-md px-2.5 py-[3px] whitespace-nowrap">최대 1,500 byte</span>
          </div>
          <div class="flex flex-col gap-2">
            <div v-for="act in subjectActivities" :key="act.name" class="flex items-start gap-2.5">
              <span class="w-2 h-2 rounded-full bg-amber flex-shrink-0 mt-[5px] opacity-70"/>
              <div>
                <div class="text-[15px] font-medium text-[#c8d8f0] leading-[1.5]">{{ act.name }}</div>
                <div class="text-sm text-[#5a7090] mt-[2px]">{{ act.desc }}</div>
              </div>
            </div>
          </div>
          <div class="text-sm text-[#6a8aaa] pt-3 border-t border-line leading-[1.75]">
            세 활동의 문장을 합치면 <strong class="text-amber font-semibold">수학 세특 영역</strong>이 완성됩니다.
          </div>
        </div>

        <div
            class="text-[15px] text-[#7aa5c8] bg-amber/[0.05] rounded-lg px-[18px] py-3.5 leading-[1.75]"
            style="border: 1px solid rgba(245,158,11,0.18); border-left: 3px solid #f59e0b;"
        >
          과목별로 프로젝트를 따로 만들거나, 한 프로젝트 안에 여러 과목을 영역으로 묶어 관리할 수 있습니다.
          학생 수가 많을수록 격자형 입력 화면이 큰 도움이 됩니다.
        </div>
      </div>

    </div>
  </div>
</template>
