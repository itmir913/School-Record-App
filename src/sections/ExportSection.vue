<script setup>
import {computed, onMounted, ref} from 'vue'
import {save} from '@tauri-apps/plugin-dialog'
import {useAreaStore} from '../stores/area.js'
import {useRecordStore} from '../stores/record.js'
import {useFileStore} from '../stores/file.js'
import {revealItemInDir} from '@tauri-apps/plugin-opener'
import {Workbook} from 'exceljs'
import WizardLayout from '../components/WizardLayout.vue'

// ── 스토어 ────────────────────────────────────────────────────

const areaStore = useAreaStore()
const recordStore = useRecordStore()
const fileStore = useFileStore()

// ── 상태 ──────────────────────────────────────────────────────

const step = ref(1)
const exportType = ref(null)
const selectedAreaId = ref(null)
const gridData = ref(null)

const exporting = ref(false)
const exportResult = ref(null)
const exportError = ref('')
const isNavigating = ref(false)

// ── 초기 데이터 로드 ──────────────────────────────────────────

onMounted(async () => {
  try {
    await areaStore.fetchAreas()
  } catch (e) {
    exportError.value = `영역 목록을 불러오지 못했습니다: ${e}`
  }
})

// ── Computed ──────────────────────────────────────────────────

const selectedArea = computed(() => areaStore.areas.find(a => a.id === selectedAreaId.value))

const canGoNext = computed(() => {
  if (step.value === 1) return exportType.value !== null
  if (step.value === 2) return selectedAreaId.value !== null
  return false
})

const estimatedRows = computed(() => {
  if (!gridData.value) return 0
  const {students, activities} = gridData.value
  if (exportType.value === 'A') return students.length * activities.length
  return students.length
})

const typeLabel = computed(() => {
  if (exportType.value === 'A') return 'A타입 — 행 단위 활동 형식'
  if (exportType.value === 'B') return 'B타입 — 열 단위 활동 형식'
  if (exportType.value === 'C') return 'C타입 — 나이스(NEIS) 문장 형식'
  return ''
})

// ── 네비게이션 ────────────────────────────────────────────────

async function goNext() {
  if (isNavigating.value) return
  isNavigating.value = true
  try {
    if (step.value === 2) {
      gridData.value = await recordStore.fetchAreaGrid(selectedAreaId.value)
    }
    step.value++
  } catch (e) {
    exportError.value = `데이터를 불러오지 못했습니다: ${e}`
  } finally {
    isNavigating.value = false
  }
}

function goPrev() {
  step.value--
}

function resetWizard() {
  step.value = 1
  exportType.value = null
  selectedAreaId.value = null
  gridData.value = null
  exportResult.value = null
  exportError.value = ''
  isNavigating.value = false
}

// ── 유틸 ──────────────────────────────────────────────────────

function bufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  const chunk = 8192
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(binary)
}

// ── 내보내기 실행 ─────────────────────────────────────────────

function normalizeContent(text) {
  return text
      .replace(/\n+/g, ' ')
      .replace(/ {2,}/g, ' ')
      .trim()
}

async function doExport() {
  if (!gridData.value) return
  exportError.value = ''
  exporting.value = true

  try {
    const {activities, students, records} = gridData.value
    let rows

    if (exportType.value === 'A') {
      rows = students.flatMap(s =>
          activities.map(a => ({
            학년: s.grade,
            반: s.class_num,
            번호: s.number,
            이름: s.name,
            활동명: a.name,
            활동내용: normalizeContent(records.find(r => r.student_id === s.id && r.activity_id === a.id)?.content ?? ''),
          }))
      )
    } else if (exportType.value === 'B') {
      rows = students.map(s => ({
        학년: s.grade,
        반: s.class_num,
        번호: s.number,
        이름: s.name,
        ...Object.fromEntries(
            activities.map(a => [
              a.name,
              normalizeContent(records.find(r => r.student_id === s.id && r.activity_id === a.id)?.content ?? ''),
            ])
        ),
      }))
    } else {
      const areaCol = selectedArea.value?.name ?? '영역'
      rows = students.map(s => ({
        학년: s.grade,
        반: s.class_num,
        번호: s.number,
        이름: s.name,
        [areaCol]: activities
            .map(a => normalizeContent(records.find(r => r.student_id === s.id && r.activity_id === a.id)?.content ?? ''))
            .filter(c => c !== '')
            .join(' '),
      }))
    }

    const areaName = selectedArea.value?.name ?? '내보내기'
    const filePath = await save({
      defaultPath: `${areaName}_내보내기.xlsx`,
      filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
    })

    if (!filePath) {
      exporting.value = false
      return
    }

    const workbook = new Workbook()
    const worksheet = workbook.addWorksheet('기록')
    if (rows.length > 0) {
      const headers = Object.keys(rows[0])

      // A~D열: 학년/반/번호/이름 (좁게), E열~: 내용 열 (넓게 + wrapText)
      const fixedWidths = [8, 8, 8, 12]
      fixedWidths.forEach((w, i) => {
        worksheet.getColumn(i + 1).width = w
      })
      for (let i = fixedWidths.length; i < headers.length; i++) {
        worksheet.getColumn(i + 1).width = exportType.value === 'A' && i === fixedWidths.length ? 22 : 60
      }

      const headerRow = worksheet.addRow(headers)
      headerRow.font = {name: '맑은 고딕', bold: true}
      headerRow.alignment = {vertical: 'middle', horizontal: 'center', wrapText: false}

      for (const row of rows) {
        const excelRow = worksheet.addRow(headers.map(h => row[h]))
        excelRow.alignment = {vertical: 'top'}
        for (let col = fixedWidths.length + 1; col <= headers.length; col++) {
          excelRow.getCell(col).alignment = {vertical: 'top', wrapText: true}
        }
      }
    }
    const buffer = await workbook.xlsx.writeBuffer()
    const data = bufferToBase64(buffer)
    await fileStore.writeBytesFile(filePath, data)

    exportResult.value = {
      fileName: filePath.split(/[\\/]/).pop(),
      filePath,
      rowCount: rows.length,
    }
    step.value++
  } catch (e) {
    exportError.value = String(e)
  } finally {
    exporting.value = false
  }
}
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden box-border">

    <!-- 헤더 -->
    <div class="flex items-center justify-between px-10 pt-9 pb-6 border-b border-line shrink-0">
      <div class="flex flex-col">
        <h2 class="text-[22px] font-bold text-ink m-0 mb-1.5">데이터 내보내기(Export)</h2>
        <p class="text-base text-ink-3 m-0">완성된 학교생활기록부를 다양한 형식으로 내보냅니다.</p>
      </div>
    </div>

    <WizardLayout
        :stepCount="3"
        :currentStep="step"
        :canGoNext="canGoNext"
        :isNavigating="isNavigating"
        :showFooter="!exportResult"
        @prev="goPrev"
        @next="goNext"
    >

      <!-- Step 1: 내보내기 유형 선택 -->
      <div v-if="step === 1">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 1. 내보내기(Export) 형식 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">내보낼 엑셀 파일의 유형을 선택하세요.</p>

        <div class="grid gap-4" style="grid-template-columns: repeat(auto-fit, minmax(max(300px, calc(50% - 8px)), 1fr))">

          <!-- A 타입 -->
          <div
              class="border-2 rounded-xl p-5 cursor-pointer transition-[border-color,background-color] duration-200"
              :class="exportType === 'A' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="exportType = 'A'"
          >
            <div class="flex items-center gap-2.5 mb-2.5">
              <span class="text-xs font-bold rounded-[6px] py-0.5 px-2 text-red bg-red/[0.12] border border-red/35">A 타입</span>
              <span class="text-base font-semibold text-ink">행 단위 활동 형식</span>
            </div>
            <p class="text-sm text-ink-5 m-0 mb-3.5 leading-relaxed">한 행에 학생 1명의 활동 1개를 기재합니다.<br>학생 1명의 활동은 여러 행에 걸쳐 기록됩니다(학생 1명 = 여러 행).</p>
            <div class="overflow-x-auto border border-line rounded-[6px]">
              <table class="border-collapse w-full text-xs">
                <thead>
                <tr>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학년</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">반</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">번호</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">이름</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">활동명</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">활동내용</th>
                </tr>
                </thead>
                <tbody>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">현장체험학습</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">지역 기관을 탐방...</td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학급자치회</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">회의에 적극 참여...</td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">2</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학생B</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">체육대회</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">다양한 종목에 참여...</td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- B 타입 -->
          <div
              class="border-2 rounded-xl p-5 cursor-pointer transition-[border-color,background-color] duration-200"
              :class="exportType === 'B' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="exportType = 'B'"
          >
            <div class="flex items-center gap-2.5 mb-2.5">
              <span class="text-xs font-bold rounded-[6px] py-0.5 px-2 text-amber bg-amber/[0.15] border border-amber/40">B 타입</span>
              <span class="text-base font-semibold text-ink">열 단위 활동 형식</span>
            </div>
            <p class="text-sm text-ink-5 m-0 mb-3.5 leading-relaxed">활동이 열(헤더)로 구분된 형식입니다.<br>학생 1명의 모든 활동이 한 행에 기록됩니다(학생 1명 = 1행).</p>
            <div class="overflow-x-auto border border-line rounded-[6px]">
              <table class="border-collapse w-full text-xs">
                <thead>
                <tr>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학년</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">반</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">번호</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">이름</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">현장체험학습</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학급자치회</th>
                </tr>
                </thead>
                <tbody>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">지역 기관을...</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">회의에 적극...</td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">2</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생B</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap"></td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap"></td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">5</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학생E</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap"></td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학급 행사 준비...</td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- C 타입 (전 너비) -->
          <div
              class="col-span-full border-2 rounded-xl p-5 cursor-pointer transition-[border-color,background-color] duration-200"
              :class="exportType === 'C' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="exportType = 'C'"
          >
            <div class="flex items-center gap-2.5 mb-2.5">
              <span class="text-xs font-bold rounded-[6px] py-0.5 px-2 text-green bg-green/[0.12] border border-green/30">C 타입</span>
              <span class="text-base font-semibold text-ink">최종 나이스(NEIS) 문장 형식 (추천)</span>
            </div>
            <p class="text-sm text-ink-5 m-0 mb-3.5 leading-relaxed">학생의 모든 활동 기록을 하나의 문장으로 결합해 내보냅니다.<br>
              나이스(NEIS) 입력용 최종 문장 생성에 사용합니다.</p>
            <div class="overflow-x-auto border border-line rounded-[6px]">
              <table class="border-collapse w-full text-xs">
                <thead>
                <tr>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학년</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">반</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">번호</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">이름</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">영역명</th>
                </tr>
                </thead>
                <tbody>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">지역 기관을 탐방... 회의에 적극 참여... 학교 행사 기획에 참여... 경기와 응원에 적극 참여...</td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">2</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생B</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">다양한 종목에 참여...</td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생C</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">도서를 바탕으로 자신의 생각... 실험 전 안전 점검을 철저히...</td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">5</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생E</td>
                  <td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학급 행사 준비 과정에서...</td>
                </tr>
                <tr>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">3</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">1</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">7</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학생G</td>
                  <td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">현장 경험을 통해 배운 내용을 바탕으로... 안전 수칙을 준수하며...</td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>

        </div>
      </div>

      <!-- Step 2: 영역 선택 -->
      <div v-else-if="step === 2">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 2. 영역(Area) 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">내보낼 영역을 선택하세요.</p>

        <p v-if="exportError && areaStore.areas.length === 0" class="text-sm text-red mt-3">{{ exportError }}</p>
        <p v-else-if="areaStore.areas.length === 0" class="text-base text-ink-5 m-0">등록된 영역이 없습니다.</p>

        <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
          <div
              v-for="area in areaStore.areas"
              :key="area.id"
              class="border-2 rounded-[10px] px-5 py-4 cursor-pointer transition-[border-color,background-color] duration-200 flex flex-col gap-1.5"
              :class="selectedAreaId === area.id ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="selectedAreaId = area.id"
          >
            <span class="text-base font-semibold text-ink">{{ area.name }}</span>
            <span class="text-sm text-ink-5">활동 {{ area.activities.length }}개</span>
          </div>
        </div>
      </div>

      <!-- Step 3: 확인 및 내보내기 -->
      <div v-else-if="step === 3">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 3. 내보내기 실행</h3>

        <div v-if="!exportResult">
          <!-- 요약 -->
          <div class="border border-line rounded-[10px] overflow-hidden mb-6">
            <div class="grid border-b border-line/70 py-[11px] px-4 last:border-b-0" style="grid-template-columns: 140px 1fr">
              <span class="text-sm text-ink-5">유형</span>
              <span class="text-sm text-ink-2">{{ typeLabel }}</span>
            </div>
            <div class="grid border-b border-line/70 py-[11px] px-4 last:border-b-0" style="grid-template-columns: 140px 1fr">
              <span class="text-sm text-ink-5">영역</span>
              <span class="text-sm text-ink-2">{{ selectedArea?.name }}</span>
            </div>
            <div class="grid border-b border-line/70 py-[11px] px-4 last:border-b-0" style="grid-template-columns: 140px 1fr">
              <span class="text-sm text-ink-5">학생 수</span>
              <span class="text-sm text-ink-2">{{ gridData?.students.length ?? 0 }}명</span>
            </div>
            <div class="grid border-b border-line/70 py-[11px] px-4 last:border-b-0" style="grid-template-columns: 140px 1fr">
              <span class="text-sm text-ink-5">활동 수</span>
              <span class="text-sm text-ink-2">{{ gridData?.activities.length ?? 0 }}개</span>
            </div>
            <div class="grid py-[11px] px-4" style="grid-template-columns: 140px 1fr">
              <span class="text-sm text-ink-5">예상 행 수</span>
              <span class="text-sm text-ink-2">{{ estimatedRows }}행 (헤더 제외)</span>
            </div>
          </div>

          <!-- 면책 안내문 -->
          <div class="flex gap-2.5 items-start mt-3.5 mb-6 px-3.5 py-3 border border-amber/30 bg-amber/[0.08] rounded-lg">
            <span class="text-amber shrink-0 mt-[3px] text-sm inline-flex items-center justify-center w-[22px] h-[22px] border border-amber/30 rounded-full">ℹ</span>
            <div class="flex flex-col gap-1.5">
              <p class="m-0 text-base leading-relaxed text-ink-2">
                이 파일은 학교생활기록부 작성을 돕기 위한 <strong>참고용 자료</strong>입니다.<br>
                반드시 <u><strong>담당 선생님께서 내용을 직접 검토</strong></u>하신 후 나이스(NEIS)에 입력해 주시기를
                부탁드립니다.
              </p>
              <p class="m-0 text-sm text-ink-4">내보내기를 실행하면 위 안내 사항을 확인하신 것으로 간주합니다.</p>
            </div>
          </div>

          <p v-if="exportError" class="text-sm text-red mt-3">{{ exportError }}</p>

          <button
              class="py-2.5 px-7 bg-blue/[0.15] border border-blue/40 rounded-lg text-blue-2 text-base font-semibold cursor-pointer transition-colors enabled:hover:bg-blue/25 disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="exporting"
              @click="doExport"
          >
            {{ exporting ? '내보내는 중...' : '내보내기 실행' }}
          </button>
        </div>
      </div>

      <!-- 완료 화면 -->
      <div v-else-if="step===4" class="flex flex-col items-center gap-4 py-12">
        <div class="text-[40px] text-green">✓</div>
        <p class="text-xl font-bold text-ink m-0">내보내기 완료</p>
        <div class="flex gap-8">
          <div class="flex flex-col items-center gap-1">
            <span class="text-[28px] font-bold text-blue-2">{{ exportResult.rowCount }}</span>
            <span class="text-sm text-ink-5">행 저장됨</span>
          </div>
        </div>
        <p class="text-sm text-ink-5 m-0">{{ exportResult.fileName }}</p>
        <div class="flex gap-2.5 mt-2">
          <button
              class="py-[9px] px-6 bg-blue/[0.12] border border-blue/35 rounded-lg text-blue-2 text-base cursor-pointer transition-colors hover:bg-blue/[0.22] hover:text-ink-2"
              @click="revealItemInDir(exportResult.filePath)"
          >파일 확인</button>
          <button
              class="py-[9px] px-6 bg-transparent border border-line rounded-lg text-ink-5 text-base cursor-pointer transition-colors hover:bg-line hover:text-ink-3"
              @click="resetWizard"
          >새로 내보내기</button>
        </div>
      </div>

    </WizardLayout>

  </div>
</template>
