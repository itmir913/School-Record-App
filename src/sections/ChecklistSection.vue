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

// ── 내보내기 실행 ─────────────────────────────────────────────

async function doExport() {
  if (!gridData.value) return
  exportError.value = ''
  exporting.value = true

  try {
    const {activities, students, records} = gridData.value

    const workbook = new Workbook()
    const worksheet = workbook.addWorksheet('체크리스트')

    // A4 가로 레이아웃, 가로 1페이지 맞춤
    worksheet.pageSetup = {
      paperSize: 9,
      orientation: 'landscape',
      fitToPage: true,
      fitToWidth: 1,
      fitToHeight: 0,
    }

    const activityNames = activities.map(a => a.name)

    for (let i = 0; i < students.length; i++) {
      const s = students[i]

      // 헤더 행: 학년/반/번호/이름 + 활동명 목록
      worksheet.addRow(['학년', '반', '번호', '이름', ...activityNames])

      // 데이터 행: 학생 정보 + O/X
      const dataRow = worksheet.addRow([
        s.grade,
        s.class_num,
        s.number,
        s.name,
        ...activities.map(a => {
          const rec = records.find(r => r.student_id === s.id && r.activity_id === a.id)
          return rec?.content?.trim() ? 'O' : 'X'
        }),
      ])

      // 마지막 학생 제외, 데이터 행 아래에 페이지 나누기 삽입
      if (i < students.length - 1) {
        dataRow.addPageBreak()
      }
    }

    // ArrayBuffer → base64 변환
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

        <div class="info-box">
          <p class="info-text">기재 내용이 있으면 <strong>O(참여함)</strong>, 없으면 <strong>X(참여하지 않음)</strong>으로 표시됩니다.</p>
          <p class="info-text">학생 1명당 1페이지(A4 가로)로 페이지 나누기가 설정됩니다.</p>
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
