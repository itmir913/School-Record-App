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
const selectedAreaId = ref(null)
const gridData = ref(null)
const previewEnabled = ref(true)
const previewRows = ref([])  // { studentId, grade, classNum, number, name, activityId, activityName, hasContent, topic }

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
  if (step.value === 1) return selectedAreaId.value !== null
  if (step.value === 2) return true
  return false
})

// previewRows를 학생 단위로 그룹핑
const previewGroups = computed(() => {
  const map = new Map()
  for (const row of previewRows.value) {
    if (!map.has(row.studentId)) {
      map.set(row.studentId, {
        studentId: row.studentId,
        grade: row.grade,
        classNum: row.classNum,
        number: row.number,
        name: row.name,
        rows: [],
      })
    }
    map.get(row.studentId).rows.push(row)
  }
  return [...map.values()]
})

// ── 네비게이션 ────────────────────────────────────────────────

async function goNext() {
  if (isNavigating.value) return
  isNavigating.value = true
  try {
    if (step.value === 1) {
      gridData.value = await recordStore.fetchAreaGrid(selectedAreaId.value)
      const {activities, students, records} = gridData.value
      previewRows.value = students.flatMap(student =>
          activities.map(activity => {
            const rec = records.find(r => r.student_id === student.id && r.activity_id === activity.id)
            const content = rec?.content?.trim() ?? ''
            return {
              studentId: student.id,
              grade: student.grade,
              classNum: student.class_num,
              number: student.number,
              name: student.name,
              activityId: activity.id,
              activityName: activity.name,
              hasContent: !!content,
              topic: content ? extractTopic(content) : '',
            }
          })
      )
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
  selectedAreaId.value = null
  gridData.value = null
  previewRows.value = []
  previewEnabled.value = true
  exportResult.value = null
  exportError.value = ''
  isNavigating.value = false
}

// ── 활동주제 추출 ─────────────────────────────────────────────

function extractTopic(content) {
  if (!content?.trim()) return ''

  // 1) 첫 문장 추출
  // - s 플래그 제거: .이 줄바꿈을 넘지 않도록
  // - m 플래그 추가: $가 각 줄 끝과 매칭
  // - \s*$ : 온점 뒤 공백만 남은 경우도 첫 문장으로 인정
  const sentenceMatch = content.match(
      /^(.+?[.!?][“”‘’"']?)(?=\s+[A-Z가-힣]|\s*$)/m
  )

  const firstSentence = sentenceMatch
      ? sentenceMatch[1].trim()
      : content.split(/\r?\n/)[0].slice(0, 100).trim()

  // 2) 따옴표 내용 전부 수집
  // 열기: " (U+0022) ' (U+0027) " " ' ' 「 『
  // 닫기: 위 + 」(U+300D) 』(U+300F) (「→」, 『→』 대응)
  const matches = [
    ...firstSentence.matchAll(
        /["'“”‘’「『]([^"'“”‘’」』]{1,120})["'“”‘’」』]/g
    )
  ]

  const values = matches
      .map(m => m[1].trim())
      .filter(Boolean)

  // 3) 중첩 제거 (부분 포함 제거)
  const filtered = values.filter((val, i, arr) =>
      !arr.some((other, j) =>
          i !== j && other.includes(val)
      )
  )

  // 4) 결과 반환
  if (filtered.length > 0) {
    return filtered.slice(0, 5).join(', ')
  }

  // 5) fallback
  const trimmed = firstSentence.slice(0, 100).trim()
  return trimmed + (firstSentence.length > 100 ? '…' : '')
}

// ── 내보내기 헬퍼 ────────────────────────────────────────────

const THIN = {style: 'thin'}
const BORDER_ALL = {top: THIN, left: THIN, bottom: THIN, right: THIN}

function styleCell(cell, {size = 12, bold = false, fill = null, wrapText = false} = {}) {
  cell.font = {name: '맑은 고딕', size, bold}
  cell.alignment = {vertical: 'middle', horizontal: 'left', wrapText}
  if (fill) cell.fill = {type: 'pattern', pattern: 'solid', fgColor: {argb: fill}}
  cell.border = BORDER_ALL
}

// studentRows: 해당 학생의 previewRow 배열
function addStudentBlock(worksheet, student, studentRows, showPreview) {
  // ── 학생 정보 4행 ──────────────────────────────────────────
  const infoData = [
    ['학년', student.grade],
    ['반', student.class_num],
    ['번호', student.number],
    ['이름', student.name],
  ]
  for (const [label, value] of infoData) {
    const row = worksheet.addRow([label, value])
    row.height = 24
    styleCell(row.getCell(1))
    styleCell(row.getCell(2))
  }

  // ── 빈 구분 행 ────────────────────────────────────────────
  const sepRow = worksheet.addRow([])
  sepRow.height = 6

  // ── 활동 헤더 행 ──────────────────────────────────────────
  const headerRow = worksheet.addRow(
      showPreview ? ['활동명', '참여여부', '활동주제'] : ['활동명', '참여여부'],
  )
  headerRow.height = 24
  styleCell(headerRow.getCell(1), {size: 13, bold: true, fill: 'FFD9D9D9'})
  styleCell(headerRow.getCell(2), {size: 13, bold: true, fill: 'FFD9D9D9'})
  if (showPreview) styleCell(headerRow.getCell(3), {size: 13, bold: true, fill: 'FFD9D9D9'})

  // ── 활동 데이터 행 (행 높이 미지정 → Excel 자동 조절) ─────
  for (const pr of studentRows) {
    const ox = pr.hasContent ? 'O' : 'X'
    const topic = showPreview ? (pr.topic ?? '') : ''
    const row = worksheet.addRow(
        showPreview ? [pr.activityName, ox, topic] : [pr.activityName, ox],
    )
    styleCell(row.getCell(1), {wrapText: true})       // 활동명: 줄 바꿈
    styleCell(row.getCell(2))
    if (showPreview) styleCell(row.getCell(3), {wrapText: true}) // 활동주제: 줄 바꿈
  }

  // ── 서명 행 (페이지 나누기 기준점) ────────────────────────
  const signRow = worksheet.addRow(
      showPreview ? ['학생 서명', '', ''] : ['학생 서명', ''],
  )
  signRow.height = 30
  styleCell(signRow.getCell(1), {bold: true})
  styleCell(signRow.getCell(2))
  if (showPreview) styleCell(signRow.getCell(3))

  // ── 빈 구분 행 ────────────────────────────────────────────
  const finalRow = worksheet.addRow([])
  finalRow.height = 6

  return finalRow
}

// ── 내보내기 실행 ─────────────────────────────────────────────

async function doExport() {
  if (!gridData.value) return
  exportError.value = ''
  exporting.value = true

  try {
    const {students} = gridData.value
    const showPreview = previewEnabled.value

    const workbook = new Workbook()
    const worksheet = workbook.addWorksheet('체크리스트')

    if (showPreview) {
      worksheet.getColumn(1).width = 26
      worksheet.getColumn(2).width = 12
      worksheet.getColumn(3).width = 40
    } else {
      worksheet.getColumn(1).width = 38
      worksheet.getColumn(2).width = 16
    }

    worksheet.pageSetup = {
      paperSize: 9,
      orientation: 'portrait',
      fitToPage: true,
      fitToWidth: 1,
      fitToHeight: 0,   // 수동 페이지 나누기로 학생 단위 분리
      margins: {left: 0.5, right: 0.5, top: 0.5, bottom: 0.5, header: 0, footer: 0},
    }

    for (let i = 0; i < students.length; i++) {
      const student = students[i]
      const studentRows = previewRows.value.filter(r => r.studentId === student.id)
      const lastRow = addStudentBlock(worksheet, student, studentRows, showPreview)
      if (i < students.length - 1) {
        lastRow.addPageBreak()
      }
    }

    const buffer = await workbook.xlsx.writeBuffer()
    const bytes = new Uint8Array(buffer)
    let binary = ''
    for (let i = 0; i < bytes.length; i++) {
      binary += String.fromCharCode(bytes[i])
    }
    const data = btoa(binary)

    const areaName = selectedArea.value?.name ?? '체크리스트'
    const filePath = await save({
      defaultPath: `${areaName}_체크리스트.xlsx`,
      filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
    })

    if (!filePath) {
      exporting.value = false
      return
    }

    await fileStore.writeBytesFile(filePath, data)

    exportResult.value = {
      fileName: filePath.split(/[\\/]/).pop(),
      filePath,
      pageCount: students.length,
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
    <div class="flex items-center justify-between px-10 pt-9 pb-6 border-b border-line flex-shrink-0">
      <div class="flex flex-col">
        <h2 class="text-[22px] font-bold text-ink m-0 mb-1.5">체크리스트 내보내기(Checklist Export)</h2>
        <p class="text-base text-ink-3 m-0">학교생활기록부 점검을 위해 활동(Activity)별 키워드 혹은 주제를 추출하여 내보냅니다.</p>
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

      <!-- Step 1: 영역 선택 -->
      <div v-if="step === 1">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 1. 영역(Area) 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">체크리스트를 만들 영역을 선택하세요.</p>

        <p v-if="exportError && areaStore.areas.length === 0" class="text-sm text-red mt-3 m-0">{{ exportError }}</p>
        <p v-else-if="areaStore.areas.length === 0" class="text-base text-ink-5 m-0 mb-6">등록된 영역이 없습니다.</p>

        <div v-else class="grid gap-3 mb-6" style="grid-template-columns: repeat(auto-fill, minmax(220px, 1fr))">
          <div
              v-for="area in areaStore.areas"
              :key="area.id"
              :class="[
                'border-2 rounded-btn px-5 py-4 cursor-pointer transition-[border-color,background-color] flex flex-col gap-1.5',
                selectedAreaId === area.id
                  ? 'border-blue/70 bg-blue/[0.06]'
                  : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'
              ]"
              @click="selectedAreaId = area.id"
          >
            <span class="text-base font-semibold text-ink">{{ area.name }}</span>
            <span class="text-sm text-ink-5">활동 {{ area.activities.length }}개</span>
          </div>
        </div>

        <!-- 한 줄 미리보기 토글 -->
        <div
            class="flex items-center justify-between gap-4 border border-line rounded-btn px-4.5 py-3.5 mb-4 cursor-pointer transition-[border-color,background-color] hover:border-blue/40 hover:bg-blue/[0.03]"
            @click="previewEnabled = !previewEnabled"
        >
          <div class="flex flex-col gap-1">
            <span class="text-base font-semibold text-ink">한 줄 미리보기</span>
            <span class="text-sm text-ink-5 leading-relaxed">O(참여함)인 활동에 한해 생기부 첫 문장의 활동주제를 추출해 C열에 표시합니다.</span>
          </div>
          <div :class="['flex-shrink-0 w-[42px] h-6 rounded-full relative transition-colors border', previewEnabled ? 'bg-blue/60 border-blue/80' : 'bg-line border-line-2']">
            <div :class="['absolute top-[3px] w-4 h-4 rounded-full transition-all duration-200', previewEnabled ? 'left-[21px] bg-blue-2' : 'left-[3px] bg-ink-4']"/>
          </div>
        </div>

        <!-- 안내 박스 -->
        <div class="border border-green/20 rounded-btn bg-green/[0.04] px-4.5 py-3.5 flex flex-col gap-1.5">
          <p class="text-sm text-green/75 m-0 leading-relaxed">기재 내용이 있으면 <strong class="text-green">O(참여함)</strong>, 없으면 <strong class="text-green">X(참여하지 않음)</strong>으로 표시됩니다.</p>
          <p class="text-sm text-green/75 m-0 leading-relaxed">학생 1명당 1페이지(A4 세로)로 페이지 나누기가 설정됩니다.</p>
        </div>
      </div>

      <!-- Step 2: 미리보기 & 편집 -->
      <div v-else-if="step === 2">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 2. 미리보기 &amp; 편집</h3>
        <p class="text-base text-ink-5 m-0 mb-6">
          활동 참여 여부를 확인하고
          <template v-if="previewEnabled">추출된 활동주제를 직접 수정한 뒤</template>
          내보내기 단계로 이동하세요.
        </p>

        <div class="border border-line rounded-btn overflow-hidden">
          <table class="w-full border-collapse text-sm">
            <thead>
            <tr>
              <th class="px-3.5 py-2.5 bg-base text-ink-4 font-semibold text-left border-b border-line sticky top-0 z-[1]">활동명</th>
              <th class="px-3.5 py-2.5 bg-base text-ink-4 font-semibold text-left border-b border-line sticky top-0 z-[1]">참여여부</th>
              <th v-if="previewEnabled" class="px-3.5 py-2.5 bg-base text-ink-4 font-semibold text-left border-b border-line sticky top-0 z-[1]">활동주제</th>
            </tr>
            </thead>
            <tbody>
            <template v-for="group in previewGroups" :key="group.studentId">
              <tr class="bg-blue/[0.07]">
                <td :colspan="previewEnabled ? 3 : 2" class="px-3.5 py-2 text-sm text-blue-2 border-t border-blue/20 border-b border-blue/12">
                  {{ group.grade }}학년 {{ group.classNum }}반 {{ group.number }}번 &nbsp;
                  <strong class="font-bold text-blue-2">{{ group.name }}</strong>
                </td>
              </tr>
              <tr v-for="row in group.rows" :key="row.activityId">
                <td :class="['px-3.5 py-[7px] border-b border-line/60 text-ink align-middle min-w-[100px]', !row.hasContent && 'opacity-[0.45]']">
                  {{ row.activityName }}
                </td>
                <td :class="['px-3.5 py-[7px] border-b border-line/60 align-middle text-center font-bold w-[60px]', !row.hasContent && 'opacity-[0.45]', row.hasContent ? 'text-green' : 'text-ink-4']">
                  {{ row.hasContent ? 'O' : 'X' }}
                </td>
                <td v-if="previewEnabled" :class="['px-3.5 py-[7px] border-b border-line/60 align-middle min-w-[180px]', !row.hasContent && 'opacity-[0.45]']">
                  <textarea
                      v-if="row.hasContent"
                      v-model="row.topic"
                      class="w-full bg-surface border border-line-2 rounded-md px-2.5 py-[5px] text-ink text-sm outline-none transition-colors box-border leading-[1.4] min-h-5 resize-y whitespace-pre-wrap focus:border-blue/60 placeholder:text-ink-4"
                      rows="2"
                      placeholder="활동주제 입력…"
                  />
                </td>
              </tr>
            </template>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Step 3: 내보내기 실행 -->
      <div v-else-if="step === 3">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 3. 내보내기 실행</h3>

        <div v-if="!exportResult">
          <!-- 요약 박스 -->
          <div class="border border-line rounded-btn overflow-hidden mb-6">
            <div class="grid gap-3 px-4 py-[11px] border-b border-line/70 last:border-b-0" style="grid-template-columns: 160px 1fr">
              <span class="text-sm text-ink-5">영역</span>
              <span class="text-sm text-ink-2">{{ selectedArea?.name }}</span>
            </div>
            <div class="grid gap-3 px-4 py-[11px] border-b border-line/70 last:border-b-0" style="grid-template-columns: 160px 1fr">
              <span class="text-sm text-ink-5">학생 수</span>
              <span class="text-sm text-ink-2">{{ gridData?.students.length ?? 0 }}명</span>
            </div>
            <div class="grid gap-3 px-4 py-[11px] border-b border-line/70 last:border-b-0" style="grid-template-columns: 160px 1fr">
              <span class="text-sm text-ink-5">활동 수</span>
              <span class="text-sm text-ink-2">{{ gridData?.activities.length ?? 0 }}개</span>
            </div>
            <div class="grid gap-3 px-4 py-[11px] border-b border-line/70 last:border-b-0" style="grid-template-columns: 160px 1fr">
              <span class="text-sm text-ink-5">예상 페이지 수</span>
              <span class="text-sm text-ink-2">{{ gridData?.students.length ?? 0 }}페이지 (학생 1명 = 1페이지)</span>
            </div>
            <div class="grid gap-3 px-4 py-[11px] border-b border-line/70 last:border-b-0" style="grid-template-columns: 160px 1fr">
              <span class="text-sm text-ink-5">한 줄 미리보기</span>
              <span class="text-sm text-ink-2">{{ previewEnabled ? '활성화 (활동주제 포함)' : '비활성화' }}</span>
            </div>
          </div>

          <!-- 면책 안내 박스 -->
          <div class="flex gap-2.5 items-start mt-3.5 mb-6 px-3.5 py-3 border border-amber/30 bg-amber/[0.08] rounded-lg">
            <span class="text-amber flex-shrink-0 mt-[3px] text-sm inline-flex items-center justify-center w-[22px] h-[22px] border border-amber/30 rounded-full">ℹ</span>
            <div class="flex flex-col gap-1.5">
              <p class="m-0 text-base text-ink-2 leading-relaxed">
                이 파일은 학교생활기록부 작성을 돕기 위한 <strong>참고용 자료</strong>입니다.<br>
                반드시 <u><strong>담당 선생님께서 내용을 직접 검토</strong></u>하신 후 나이스(NEIS)에 입력해 주시기를
                부탁드립니다.
              </p>
              <p class="m-0 text-sm text-ink-4">내보내기를 실행하면 위 안내 사항을 확인하신 것으로 간주합니다.</p>
            </div>
          </div>

          <p v-if="exportError" class="text-sm text-red mt-3 m-0">{{ exportError }}</p>

          <button
              class="px-7 py-2.5 bg-blue/15 border border-blue/40 rounded-lg text-blue-2 text-base font-semibold cursor-pointer transition-[background-color,color] enabled:hover:bg-blue/25 enabled:hover:text-ink-2 disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="exporting"
              @click="doExport"
          >
            {{ exporting ? '내보내는 중...' : '체크리스트 내보내기' }}
          </button>
        </div>
      </div>

      <!-- Step 4: 완료 -->
      <div v-else-if="step === 4" class="flex flex-col items-center gap-4 py-12">
        <div class="text-[40px] text-green">✓</div>
        <p class="text-xl font-bold text-ink m-0">내보내기 완료</p>
        <div class="flex gap-8">
          <div class="flex flex-col items-center gap-1">
            <span class="text-[28px] font-bold text-blue-2">{{ exportResult.pageCount }}</span>
            <span class="text-sm text-ink-5">페이지</span>
          </div>
        </div>
        <p class="text-sm text-ink-5 m-0">{{ exportResult.fileName }}</p>
        <div class="flex gap-2.5 mt-2">
          <button
              class="px-6 py-[9px] bg-blue/[0.12] border border-blue/35 rounded-lg text-blue-2 text-base cursor-pointer transition-[background-color,color] hover:bg-blue/[0.22] hover:text-ink-2"
              @click="revealItemInDir(exportResult.filePath)"
          >파일 확인</button>
          <button
              class="px-6 py-[9px] bg-transparent border border-line rounded-lg text-ink-5 text-base cursor-pointer transition-[background-color,color] hover:bg-line hover:text-ink-3"
              @click="resetWizard"
          >새로 내보내기</button>
        </div>
      </div>

    </WizardLayout>

  </div>
</template>
