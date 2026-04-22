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
const previewEnabled = ref(false)

const exporting = ref(false)
const exportResult = ref(null)
const exportError = ref('')

// ── 초기 데이터 로드 ──────────────────────────────────────────

onMounted(async () => {
  areas.value = await invoke('get_areas')
})

// ── Computed ──────────────────────────────────────────────────

const selectedArea = computed(() => areas.value.find(a => a.id === selectedAreaId.value))

const canGoNext = computed(() => selectedAreaId.value !== null)

// ── 네비게이션 ────────────────────────────────────────────────

async function goNext() {
  gridData.value = await invoke('get_area_grid', {areaId: selectedAreaId.value})
  step.value++
}

function goPrev() {
  step.value--
}

function resetWizard() {
  step.value = 1
  selectedAreaId.value = null
  gridData.value = null
  exportResult.value = null
  exportError.value = ''
}

// ── 활동주제 추출 ─────────────────────────────────────────────

function extractTopic(content) {
  if (!content?.trim()) return ''

  // 첫 문장 추출: 점(.) 기준
  const dotIdx = content.indexOf('.')
  const firstSentence = dotIdx >= 0 ? content.slice(0, dotIdx).trim() : null

  if (firstSentence) {
    // 큰/작은따옴표(직선·곡선 모두) 로 감싸진 부분 추출
    const m = firstSentence.match(/["""''「『]([^"""''」』]+)["""''」』]/u)
    if (m) return m[1]
    return firstSentence
  }

  // 점이 없는 경우: 100자 이내 일부만
  return content.trim().slice(0, 100)
}

// ── 내보내기 헬퍼 ────────────────────────────────────────────

const THIN = {style: 'thin'}
const BORDER_ALL = {top: THIN, left: THIN, bottom: THIN, right: THIN}

function styleCell(cell, {size = 12, bold = false, fill = null} = {}) {
  cell.font = {name: '맑은 고딕', size, bold}
  cell.alignment = {vertical: 'middle', horizontal: 'left'}
  if (fill) cell.fill = {type: 'pattern', pattern: 'solid', fgColor: {argb: fill}}
  cell.border = BORDER_ALL
}

function addStudentBlock(worksheet, student, activities, records, rowHeight, showPreview) {
  // ── 학생 정보 4행 ──────────────────────────────────────────
  const infoRows = [
    ['학년', student.grade],
    ['반', student.class_num],
    ['번호', student.number],
    ['이름', student.name],
  ]
  for (const [label, value] of infoRows) {
    const row = worksheet.addRow([label, value])
    row.height = rowHeight
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
  headerRow.height = rowHeight
  styleCell(headerRow.getCell(1), {size: 13, bold: true, fill: 'FFD9D9D9'})
  styleCell(headerRow.getCell(2), {size: 13, bold: true, fill: 'FFD9D9D9'})
  if (showPreview) styleCell(headerRow.getCell(3), {size: 13, bold: true, fill: 'FFD9D9D9'})

  // ── 활동 데이터 행 ────────────────────────────────────────
  for (const activity of activities) {
    const rec = records.find(r => r.student_id === student.id && r.activity_id === activity.id)
    const content = rec?.content?.trim()
    const ox = content ? 'O' : 'X'
    const topic = showPreview && content ? extractTopic(rec.content) : ''
    const row = worksheet.addRow(showPreview ? [activity.name, ox, topic] : [activity.name, ox])
    row.height = rowHeight
    styleCell(row.getCell(1))
    styleCell(row.getCell(2))
    if (showPreview) styleCell(row.getCell(3))
  }

  // ── 서명 행 (마지막 행, pageBreak 기준) ────────────────────
  const signRow = worksheet.addRow(showPreview ? ['학생 서명', '', ''] : ['학생 서명', ''])
  signRow.height = rowHeight
  styleCell(signRow.getCell(1), {bold: true})
  styleCell(signRow.getCell(2))
  if (showPreview) styleCell(signRow.getCell(3))

  return signRow
}

// ── 내보내기 실행 ─────────────────────────────────────────────

async function doExport() {
  if (!gridData.value) return
  exportError.value = ''
  exporting.value = true

  try {
    const {activities, students, records} = gridData.value

    const workbook = new Workbook()
    const worksheet = workbook.addWorksheet('체크리스트')

    if (previewEnabled.value) {
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
      fitToHeight: 0,   // 높이 제한 없음 — 수동 페이지 나누기로 학생 단위 분리
      margins: {left: 0.5, right: 0.5, top: 0.5, bottom: 0.5, header: 0, footer: 0},
    }

    // A4 세로 0.5인치 여백 기준 사용 가능 높이 ≈ 770pt
    // 비구분자 행 수: 정보(4) + 활동헤더(1) + 활동(N) + 서명(1) = N+6
    // 구분 행: 6pt 고정 → 나머지를 균등 분배
    const USABLE_PTS = 770
    const SEP_PTS = 6
    const nonSepRows = 4 + 1 + activities.length + 1
    const rowHeight = Math.max(16, Math.floor((USABLE_PTS - SEP_PTS) / nonSepRows))

    for (let i = 0; i < students.length; i++) {
      const lastRow = addStudentBlock(worksheet, students[i], activities, records, rowHeight, previewEnabled.value)
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
      <h2 class="section-title">체크리스트 내보내기</h2>
      <div class="step-indicator">
        <div v-for="n in 2" :key="n" class="step-dot"
             :class="{ 'step-dot--active': step === n, 'step-dot--done': step > n }">
          {{ step > n ? '✓' : n }}
        </div>
      </div>
    </div>

    <!-- 본문 -->
    <div class="wizard-body" ref="wizardBodyRef">

      <!-- Step 1: 영역 선택 -->
      <div v-if="step === 1" class="step-content">
        <h3 class="step-title">영역 선택</h3>
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

      <!-- Step 2: 확인 및 내보내기 -->
      <div v-else-if="step === 2" class="step-content">
        <h3 class="step-title">내보내기 실행</h3>

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
      <button v-if="step < 2" class="btn-next" :disabled="!canGoNext" @click="goNext">
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

.section-title {
  font-size: 22px;
  font-weight: 700;
  color: #e2e8f0;
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
  margin: 0;
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
