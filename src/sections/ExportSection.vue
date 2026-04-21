<script setup>
import {computed, onMounted, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'
import {ArrowLeft, ArrowRight} from 'lucide-vue-next'
import * as XLSX from 'xlsx'

// ── 상태 ──────────────────────────────────────────────────────

const step = ref(1)
const wizardBodyRef = ref(null)

watch(step, () => {
  wizardBodyRef.value?.scrollTo({top: 0, behavior: 'smooth'})
})

const exportType = ref(null)
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
  if (exportType.value === 'A') return 'A타입 — 행별 활동 형식'
  if (exportType.value === 'B') return 'B타입 — 열별 활동 형식'
  if (exportType.value === 'C') return 'C타입 — 합본 형식'
  return ''
})

// ── 네비게이션 ────────────────────────────────────────────────

async function goNext() {
  if (step.value === 2) {
    gridData.value = await invoke('get_area_grid', {areaId: selectedAreaId.value})
  }
  step.value++
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
}

// ── 내보내기 실행 ─────────────────────────────────────────────

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
            활동내용: records.find(r => r.student_id === s.id && r.activity_id === a.id)?.content ?? '',
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
              records.find(r => r.student_id === s.id && r.activity_id === a.id)?.content ?? '',
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
            .map(a => records.find(r => r.student_id === s.id && r.activity_id === a.id)?.content ?? '')
            .filter(c => c.trim() !== '')
            .join(' '),
      }))
    }

    const wb = XLSX.utils.book_new()
    const ws = XLSX.utils.json_to_sheet(rows)
    XLSX.utils.book_append_sheet(wb, ws, '기록')

    const areaName = selectedArea.value?.name ?? '내보내기'
    const filePath = await save({
      defaultPath: `${areaName}_내보내기.xlsx`,
      filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
    })

    if (!filePath) {
      exporting.value = false
      return
    }

    const data = XLSX.write(wb, {type: 'base64', bookType: 'xlsx'})
    await invoke('write_bytes_file', {path: filePath, data})

    exportResult.value = {
      fileName: filePath.split(/[\\/]/).pop(),
      rowCount: rows.length,
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
      <h2 class="section-title">데이터 내보내기</h2>
      <div class="step-indicator">
        <div v-for="n in 3" :key="n" class="step-dot"
             :class="{ 'step-dot--active': step === n, 'step-dot--done': step > n }">
          {{ step > n ? '✓' : n }}
        </div>
      </div>
    </div>

    <!-- 본문 -->
    <div class="wizard-body" ref="wizardBodyRef">

      <!-- Step 1: 내보내기 유형 선택 -->
      <div v-if="step === 1" class="step-content">
        <h3 class="step-title">내보내기 유형 선택</h3>
        <p class="step-desc">내보낼 파일의 구조를 선택하세요.</p>

        <div class="type-cards">
          <div class="type-card" :class="{ 'type-card--selected': exportType === 'A' }" @click="exportType = 'A'">
            <div class="type-card-top">
              <span class="type-badge">A 타입</span>
              <span class="type-name">행별 활동 형식</span>
            </div>
            <p class="type-desc">한 행에 학생 1명의 활동 1개가 기재됩니다.<br>기록이 없는 활동도 빈 행으로 포함됩니다.</p>
            <div class="sample-table-wrap">
              <table class="sample-table">
                <thead>
                <tr>
                  <th>학년</th>
                  <th>반</th>
                  <th>번호</th>
                  <th>이름</th>
                  <th>활동명</th>
                  <th>활동내용</th>
                </tr>
                </thead>
                <tbody>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>1</td>
                  <td>홍길동</td>
                  <td>자율활동</td>
                  <td>학급 회장으로...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>1</td>
                  <td>홍길동</td>
                  <td>동아리</td>
                  <td>로봇 동아리에서...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>2</td>
                  <td>김철수</td>
                  <td>자율활동</td>
                  <td></td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div class="type-card" :class="{ 'type-card--selected': exportType === 'B' }" @click="exportType = 'B'">
            <div class="type-card-top">
              <span class="type-badge">B 타입</span>
              <span class="type-name">열별 활동 형식</span>
            </div>
            <p class="type-desc">활동이 열(헤더)로 나뉜 형식입니다.<br>한 학생의 모든 활동이 한 행에 있습니다.</p>
            <div class="sample-table-wrap">
              <table class="sample-table">
                <thead>
                <tr>
                  <th>학년</th>
                  <th>반</th>
                  <th>번호</th>
                  <th>이름</th>
                  <th>자율활동</th>
                  <th>동아리</th>
                </tr>
                </thead>
                <tbody>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>1</td>
                  <td>홍길동</td>
                  <td>학급 회장으로...</td>
                  <td>로봇 동아리에서...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>2</td>
                  <td>김철수</td>
                  <td></td>
                  <td>독서 동아리에서...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>3</td>
                  <td>박영희</td>
                  <td>부회장으로...</td>
                  <td></td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div class="type-card type-card--wide" :class="{ 'type-card--selected': exportType === 'C' }"
               @click="exportType = 'C'">
            <div class="type-card-top">
              <span class="type-badge type-badge--c">C 타입</span>
              <span class="type-name">합본 형식 (추천)</span>
            </div>
            <p class="type-desc">모든 활동 기록을 공백으로 합쳐 영역명 열 하나에 담습니다.<br>생기부 작성용 합본 텍스트를 그대로 내보낼 때 사용합니다.</p>
            <div class="sample-table-wrap">
              <table class="sample-table">
                <thead>
                <tr>
                  <th>학년</th>
                  <th>반</th>
                  <th>번호</th>
                  <th>이름</th>
                  <th>자율활동 (영역명)</th>
                </tr>
                </thead>
                <tbody>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>1</td>
                  <td>홍길동</td>
                  <td>학급 회장으로... 로봇 동아리에서...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>2</td>
                  <td>김철수</td>
                  <td>독서 동아리에서...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>3</td>
                  <td>박영희</td>
                  <td>부회장으로... 컴퓨터 동아리에서...</td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <!-- Step 2: 영역 선택 -->
      <div v-else-if="step === 2" class="step-content">
        <h3 class="step-title">영역 선택</h3>
        <p class="step-desc">내보낼 영역을 선택하세요.</p>

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
      </div>

      <!-- Step 3: 확인 및 내보내기 -->
      <div v-else-if="step === 3" class="step-content">
        <h3 class="step-title">내보내기 실행</h3>

        <div v-if="!exportResult">
          <div class="summary-box">
            <div class="summary-row">
              <span class="summary-key">유형</span>
              <span class="summary-val">{{ typeLabel }}</span>
            </div>
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
              <span class="summary-key">예상 행 수</span>
              <span class="summary-val">{{ estimatedRows }}행 (헤더 제외)</span>
            </div>
          </div>

          <p v-if="exportError" class="error-text">{{ exportError }}</p>

          <button class="btn-export" :disabled="exporting" @click="doExport">
            {{ exporting ? '내보내는 중...' : '내보내기 실행' }}
          </button>
        </div>

        <div v-else class="result-box">
          <div class="result-check">✓</div>
          <p class="result-title">내보내기 완료</p>
          <div class="result-stats">
            <div class="stat-item">
              <span class="stat-val">{{ exportResult.rowCount }}</span>
              <span class="stat-label">행 저장됨</span>
            </div>
          </div>
          <p class="result-filename">{{ exportResult.fileName }}</p>
          <button class="btn-reset" @click="resetWizard">새로 내보내기</button>
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

.wizard-body::-webkit-scrollbar {
  width: 6px;
}

.wizard-body::-webkit-scrollbar-thumb {
  background-color: #1a2035;
  border-radius: 3px;
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

/* Step 1: 타입 카드 */
.type-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.type-card--wide {
  grid-column: 1 / -1;
}

.type-card {
  border: 2px solid #1a2035;
  border-radius: 12px;
  padding: 20px;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
}

.type-card:hover {
  border-color: rgba(59, 91, 219, 0.4);
  background-color: rgba(59, 91, 219, 0.03);
}

.type-card--selected {
  border-color: rgba(59, 91, 219, 0.7);
  background-color: rgba(59, 91, 219, 0.06);
}

.type-card-top {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.type-badge {
  font-size: 12px;
  font-weight: 700;
  color: #7ba8f0;
  background: rgba(59, 91, 219, 0.15);
  border: 1px solid rgba(59, 91, 219, 0.3);
  border-radius: 6px;
  padding: 2px 8px;
}

.type-badge--c {
  color: #86efac;
  background: rgba(52, 211, 153, 0.12);
  border-color: rgba(52, 211, 153, 0.3);
}

.type-name {
  font-size: 16px;
  font-weight: 600;
  color: #e2e8f0;
}

.type-desc {
  font-size: 14px;
  color: var(--clr-text-subtle);
  margin: 0 0 14px;
  line-height: 1.6;
}

.sample-table-wrap {
  overflow-x: auto;
  border: 1px solid #1a2035;
  border-radius: 6px;
}

.sample-table {
  border-collapse: collapse;
  width: 100%;
  font-size: 12px;
}

.sample-table th {
  padding: 6px 8px;
  background: #0a0f1e;
  color: var(--clr-text-hint);
  font-weight: 600;
  text-align: left;
  border-bottom: 1px solid #1a2035;
  white-space: nowrap;
}

.sample-table td {
  padding: 5px 8px;
  color: #7ba3d4;
  border-bottom: 1px solid rgba(26, 32, 53, 0.5);
  white-space: nowrap;
}

.sample-table tr:last-child td {
  border-bottom: none;
}

/* Step 2: 영역 카드 */
.area-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
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

/* Step 3: 요약 & 결과 */
.summary-box {
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
  margin-bottom: 24px;
}

.summary-row {
  display: grid;
  grid-template-columns: 140px 1fr;
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

.btn-reset {
  margin-top: 8px;
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
