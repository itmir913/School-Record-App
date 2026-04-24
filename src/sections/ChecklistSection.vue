<script setup>
import {computed, onMounted, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'
import {revealItemInDir} from '@tauri-apps/plugin-opener'
import {ArrowLeft, ArrowRight} from 'lucide-vue-next'
import {Workbook} from 'exceljs'

// ── 상태 ──────────────────────────────────────────────────────

const step = ref(1)
const wizardBodyRef = ref(null)

watch(step, () => {
  wizardBodyRef.value?.scrollTo({top: 0, behavior: 'smooth'})
})

const areas = ref([])
const selectedAreaId = ref(null)
const gridData = ref(null)
const previewEnabled = ref(true)
const previewRows = ref([])  // { studentId, grade, classNum, number, name, activityId, activityName, hasContent, topic }

const exporting = ref(false)
const exportResult = ref(null)
const exportError = ref('')

// ── 초기 데이터 로드 ──────────────────────────────────────────

onMounted(async () => {
  areas.value = await invoke('get_areas')
})

// ── Computed ──────────────────────────────────────────────────

const selectedArea = computed(() => areas.value.find(a => a.id === selectedAreaId.value))

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
  if (step.value === 1) {
    gridData.value = await invoke('get_area_grid', {areaId: selectedAreaId.value})
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
}

function goPrev() {
  step.value--
}

function resetWizard() {
  step.value = 1
  selectedAreaId.value = null
  gridData.value = null
  previewRows.value = []
  exportResult.value = null
  exportError.value = ''
}

// ── 활동주제 추출 ─────────────────────────────────────────────

function extractTopic(content) {
  if (!content?.trim()) return ''

  // 1) 첫 문장 추출
  // - s 플래그 제거: .이 줄바꿈을 넘지 않도록
  // - m 플래그 추가: $가 각 줄 끝과 매칭
  // - \s*$ : 온점 뒤 공백만 남은 경우도 첫 문장으로 인정
  const sentenceMatch = content.match(
      /^(.+?[.!?][\u201C\u201D\u2018\u2019\u0022\u0027]?)(?=\s+[A-Z가-힣]|\s*$)/m
  )

  const firstSentence = sentenceMatch
      ? sentenceMatch[1].trim()
      : content.split(/\r?\n/)[0].slice(0, 100).trim()

  // 2) 따옴표 내용 전부 수집
  // 열기: “ (U+0022) ‘ (U+0027) “ “ ‘ ‘ 「 『
  // 닫기: 위 + 」(U+300D) 』(U+300F) (「→」, 『→』 대응)
  const matches = [
    ...firstSentence.matchAll(
        /[\u0022\u0027\u201C\u201D\u2018\u2019\u300C\u300E]([^\u0022\u0027\u201C\u201D\u2018\u2019\u300D\u300F]{1,120})[\u0022\u0027\u201C\u201D\u2018\u2019\u300D\u300F]/g
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

    await invoke('write_bytes_file', {path: filePath, data})

    exportResult.value = {
      fileName: filePath.split(/[\\/]/).pop(),
      filePath,
      pageCount: students.length,
    }
  } catch (e) {
    exportError.value = String(e)
  } finally {
    exporting.value = false
  }
}
</script>

<template>
  <div class="section">

    <!-- 헤더 -->
    <div class="toolbar">
      <div class="section-header">
        <h2 class="section-title">체크리스트(Checklist) 내보내기</h2>
        <p class="section-desc">학교생활기록부 점검을 위해 활동(Activity)별 키워드 혹은 주제를 추출하여 내보냅니다.</p>
      </div>
      <div class="step-indicator">
        <div v-for="n in 3" :key="n" class="step-dot"
             :class="{ 'step-dot--active': step === n, 'step-dot--done': step > n }">
          {{ step > n ? '✓' : n }}
        </div>
      </div>
    </div>

    <!-- 본문 -->
    <div class="wizard-body" ref="wizardBodyRef">

      <!-- Step 1: 영역 선택 -->
      <div v-if="step === 1" class="step-content">
        <h3 class="step-title">Step 1. 영역(Area) 선택</h3>
        <p class="step-desc">체크리스트를 만들 영역을 선택하세요.</p>

        <p v-if="areas.length === 0" class="empty-hint">등록된 영역이 없습니다.</p>

        <div v-else class="area-cards">
          <div
              v-for="area in areas"
              :key="area.id"
              class="area-card"
              :class="{ 'area-card--selected': selectedAreaId === area.id }"
              @click="selectedAreaId = area.id"
          >
            <span class="area-card-name">{{ area.name }}</span>
            <span class="area-card-meta">활동 {{ area.activities.length }}개</span>
          </div>
        </div>

        <!-- 한 줄 미리보기 토글 -->
        <div class="toggle-row" @click="previewEnabled = !previewEnabled">
          <div class="toggle-label">
            <span class="toggle-title">한 줄 미리보기</span>
            <span class="toggle-desc">O(참여함)인 활동에 한해 생기부 첫 문장의 활동주제를 추출해 C열에 표시합니다.</span>
          </div>
          <div class="toggle-switch" :class="{ 'toggle-switch--on': previewEnabled }">
            <div class="toggle-knob"/>
          </div>
        </div>

        <div class="info-box">
          <p class="info-text">기재 내용이 있으면 <strong>O(참여함)</strong>, 없으면 <strong>X(참여하지 않음)</strong>으로 표시됩니다.</p>
          <p class="info-text">학생 1명당 1페이지(A4 세로)로 페이지 나누기가 설정됩니다.</p>
        </div>
      </div>

      <!-- Step 2: 미리보기 & 편집 -->
      <div v-else-if="step === 2" class="step-content">
        <h3 class="step-title">Step 2. 미리보기 & 편집</h3>
        <p class="step-desc">
          활동 참여 여부를 확인하고
          <template v-if="previewEnabled">추출된 활동주제를 직접 수정한 뒤</template>
          내보내기 단계로 이동하세요.
        </p>

        <div class="preview-table-wrap">
          <table class="preview-table">
            <thead>
            <tr>
              <th>활동명</th>
              <th>참여여부</th>
              <th v-if="previewEnabled">활동주제</th>
            </tr>
            </thead>
            <tbody>
            <template v-for="group in previewGroups" :key="group.studentId">
              <tr class="group-header-row">
                <td :colspan="previewEnabled ? 3 : 2" class="group-header-cell">
                  {{ group.grade }}학년 {{ group.classNum }}반 {{ group.number }}번 &nbsp;
                  <strong>{{ group.name }}</strong>
                </td>
              </tr>
              <tr v-for="row in group.rows" :key="row.activityId"
                  :class="{ 'row--x': !row.hasContent }">
                <td class="cell-activity">{{ row.activityName }}</td>
                <td class="cell-ox" :class="row.hasContent ? 'cell-ox--o' : 'cell-ox--x'">
                  {{ row.hasContent ? 'O' : 'X' }}
                </td>
                <td v-if="previewEnabled" class="cell-topic">
                  <textarea
                      v-if="row.hasContent"
                      v-model="row.topic"
                      class="topic-input"
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
      <div v-else-if="step === 3" class="step-content">
        <h3 class="step-title">Step 3. 내보내기 실행</h3>

        <div v-if="!exportResult">
          <div class="summary-box">
            <div class="summary-row">
              <span class="summary-key">영역</span>
              <span class="summary-val">{{ selectedArea?.name }}</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">학생 수</span>
              <span class="summary-val">{{ gridData?.students.length ?? 0 }}명</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">활동 수</span>
              <span class="summary-val">{{ gridData?.activities.length ?? 0 }}개</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">예상 페이지 수</span>
              <span class="summary-val">{{ gridData?.students.length ?? 0 }}페이지 (학생 1명 = 1페이지)</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">한 줄 미리보기</span>
              <span class="summary-val">{{ previewEnabled ? '활성화 (활동주제 포함)' : '비활성화' }}</span>
            </div>
          </div>

          <p v-if="exportError" class="error-text">{{ exportError }}</p>

          <button class="btn-export" :disabled="exporting" @click="doExport">
            {{ exporting ? '내보내는 중...' : '체크리스트 내보내기' }}
          </button>
        </div>

        <div v-else class="result-box">
          <div class="result-check">✓</div>
          <p class="result-title">내보내기 완료</p>
          <div class="result-stats">
            <div class="stat-item">
              <span class="stat-val">{{ exportResult.pageCount }}</span>
              <span class="stat-label">페이지</span>
            </div>
          </div>
          <p class="result-filename">{{ exportResult.fileName }}</p>
          <div class="result-actions">
            <button class="btn-reveal" @click="revealItemInDir(exportResult.filePath)">파일 확인</button>
            <button class="btn-reset" @click="resetWizard">새로 내보내기</button>
          </div>
        </div>
      </div>

    </div>

    <!-- 하단 네비게이션 -->
    <div v-if="!exportResult" class="wizard-footer">
      <button class="btn-prev" :disabled="step === 1" @click="goPrev">
        <ArrowLeft :size="15"/>
        이전
      </button>
      <button v-if="step < 3" class="btn-next" :disabled="!canGoNext" @click="goNext">
        다음
        <ArrowRight :size="15"/>
      </button>
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

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 36px 40px 24px;
  border-bottom: 1px solid #1a2035;
  flex-shrink: 0;
}

.section-header {
  display: flex;
  flex-direction: column;
  height: 100%;
  box-sizing: border-box;
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

.step-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
}

.step-dot {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  font-size: 13px;
  font-weight: 600;
  border: 1px solid #1a2035;
  color: var(--clr-text-hint);
  background: transparent;
  transition: all 0.2s;
}

.step-dot--active {
  border-color: rgba(59, 91, 219, 0.8);
  color: #7ba8f0;
  background: rgba(59, 91, 219, 0.12);
}

.step-dot--done {
  border-color: rgba(52, 211, 153, 0.5);
  color: #34d399;
  background: rgba(52, 211, 153, 0.08);
}

.wizard-body {
  flex: 1;
  overflow-y: auto;
  padding: 32px 40px;
}

.step-content {
}

.step-title {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0 0 6px;
}

.step-desc {
  font-size: 15px;
  color: var(--clr-text-subtle);
  margin: 0 0 24px;
}

.empty-hint {
  font-size: 15px;
  color: var(--clr-text-subtle);
  margin: 0 0 24px;
}

.error-text {
  font-size: 14px;
  color: #f87171;
  margin: 12px 0 0;
}

/* 영역 카드 */
.area-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
  margin-bottom: 24px;
}

.area-card {
  border: 2px solid #1a2035;
  border-radius: 10px;
  padding: 16px 20px;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.area-card:hover {
  border-color: rgba(59, 91, 219, 0.4);
  background-color: rgba(59, 91, 219, 0.03);
}

.area-card--selected {
  border-color: rgba(59, 91, 219, 0.7);
  background-color: rgba(59, 91, 219, 0.06);
}

.area-card-name {
  font-size: 16px;
  font-weight: 600;
  color: #e2e8f0;
}

.area-card-meta {
  font-size: 13px;
  color: var(--clr-text-subtle);
}

/* 한 줄 미리보기 토글 */
.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  border: 1px solid #1a2035;
  border-radius: 10px;
  padding: 14px 18px;
  margin-bottom: 16px;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
}

.toggle-row:hover {
  border-color: rgba(59, 91, 219, 0.4);
  background-color: rgba(59, 91, 219, 0.03);
}

.toggle-label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.toggle-title {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
}

.toggle-desc {
  font-size: 13px;
  color: var(--clr-text-subtle);
  line-height: 1.5;
}

.toggle-switch {
  flex-shrink: 0;
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: #1a2035;
  border: 1px solid #263246;
  position: relative;
  transition: background-color 0.2s;
}

.toggle-switch--on {
  background: rgba(59, 91, 219, 0.6);
  border-color: rgba(59, 91, 219, 0.8);
}

.toggle-knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #4a5568;
  transition: left 0.2s, background-color 0.2s;
}

.toggle-switch--on .toggle-knob {
  left: 21px;
  background: #93c5fd;
}

/* 안내 박스 */
.info-box {
  border: 1px solid rgba(52, 211, 153, 0.2);
  border-radius: 10px;
  background: rgba(52, 211, 153, 0.04);
  padding: 14px 18px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.info-text {
  font-size: 14px;
  color: #86c9b0;
  margin: 0;
  line-height: 1.6;
}

.info-text strong {
  color: #34d399;
}

/* 미리보기 테이블 */
.preview-table-wrap {
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
}

.preview-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.preview-table thead th {
  padding: 10px 14px;
  background: #0a0f1e;
  color: var(--clr-text-hint);
  font-weight: 600;
  text-align: left;
  border-bottom: 1px solid #1a2035;
  position: sticky;
  top: 0;
  z-index: 1;
}

.group-header-row {
  background: rgba(59, 91, 219, 0.07);
}

.group-header-cell {
  padding: 8px 14px;
  font-size: 13px;
  color: #7ba8f0;
  border-top: 1px solid rgba(59, 91, 219, 0.2);
  border-bottom: 1px solid rgba(59, 91, 219, 0.12);
}

.group-header-cell strong {
  font-weight: 700;
  color: #93c5fd;
}

.preview-table tbody tr:not(.group-header-row) td {
  padding: 7px 14px;
  border-bottom: 1px solid rgba(26, 32, 53, 0.6);
  color: #c8d8f0;
  vertical-align: middle;
}

.row--x td {
  opacity: 0.45;
}

.cell-activity {
  color: #e2e8f0;
  min-width: 100px;
}

.cell-ox {
  text-align: center;
  font-weight: 700;
  width: 60px;
}

.cell-ox--o {
  color: #34d399;
}

.cell-ox--x {
  color: #64748b;
}

.cell-topic {
  min-width: 180px;
}

.topic-input {
  width: 100%;
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid #263246;
  border-radius: 6px;
  padding: 5px 10px;
  color: #e2e8f0;
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
  box-sizing: border-box;
  line-height: 1.4;
  min-height: 20px;
  resize: vertical;
  white-space: pre-wrap;
}

.topic-input:focus {
  border-color: rgba(59, 91, 219, 0.6);
}

.topic-input::placeholder {
  color: var(--clr-text-hint);
}

/* 요약 & 결과 */
.summary-box {
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
  margin-bottom: 24px;
}

.summary-row {
  display: grid;
  grid-template-columns: 160px 1fr;
  gap: 12px;
  padding: 11px 16px;
  border-bottom: 1px solid rgba(26, 32, 53, 0.7);
}

.summary-row:last-child {
  border-bottom: none;
}

.summary-key {
  font-size: 14px;
  color: var(--clr-text-subtle);
}

.summary-val {
  font-size: 14px;
  color: #c8d8f0;
}

.btn-export {
  padding: 10px 28px;
  background: rgba(59, 91, 219, 0.15);
  border: 1px solid rgba(59, 91, 219, 0.4);
  border-radius: 8px;
  color: #7ba8f0;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.btn-export:hover:not(:disabled) {
  background: rgba(59, 91, 219, 0.25);
  color: #93c5fd;
}

.btn-export:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.result-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 48px 0;
}

.result-check {
  font-size: 40px;
  color: #34d399;
}

.result-title {
  font-size: 20px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0;
}

.result-stats {
  display: flex;
  gap: 32px;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.stat-val {
  font-size: 28px;
  font-weight: 700;
  color: #7ba8f0;
}

.stat-label {
  font-size: 13px;
  color: var(--clr-text-subtle);
}

.result-filename {
  font-size: 14px;
  color: var(--clr-text-subtle);
  margin: 0;
}

.result-actions {
  display: flex;
  gap: 10px;
  margin-top: 8px;
}

.btn-reveal {
  padding: 9px 24px;
  background: rgba(59, 91, 219, 0.12);
  border: 1px solid rgba(59, 91, 219, 0.35);
  border-radius: 8px;
  color: #7ba8f0;
  font-size: 15px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.btn-reveal:hover {
  background: rgba(59, 91, 219, 0.22);
  color: #93c5fd;
}

.btn-reset {
  padding: 9px 24px;
  background: none;
  border: 1px solid #1a2035;
  border-radius: 8px;
  color: var(--clr-text-subtle);
  font-size: 15px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.btn-reset:hover {
  background: #1a2035;
  color: #93afd4;
}

/* 하단 네비게이션 */
.wizard-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 40px;
  border-top: 1px solid #1a2035;
  flex-shrink: 0;
}

.btn-prev,
.btn-next {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 9px 18px;
  border-radius: 8px;
  border: 1px solid #1a2035;
  background: none;
  color: var(--clr-text-subtle);
  font-size: 15px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s, border-color 0.15s;
}

.btn-prev:hover:not(:disabled),
.btn-next:hover:not(:disabled) {
  background: #1a2035;
  color: #93afd4;
}

.btn-prev:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.btn-next:not(:disabled) {
  color: #7ba8f0;
  border-color: rgba(59, 91, 219, 0.3);
  background: rgba(59, 91, 219, 0.06);
}

.btn-next:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
</style>
