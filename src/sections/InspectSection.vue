<script setup>
import {computed, onMounted, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'
import {revealItemInDir} from '@tauri-apps/plugin-opener'
import {Workbook} from 'exceljs'
import {
  Check,
  ChevronRight,
  FileDown,
  Loader2,
  Plus,
  ScanSearch,
  Trash2,
  X,
} from 'lucide-vue-next'
import {useSynonymStore} from '../stores/synonymStore'
import {useAreaStore} from '../stores/area'
import {performInspection} from '../services/synonymService'
import DiffView from '../components/DiffView.vue'
import WizardLayout from '../components/WizardLayout.vue'

const store = useSynonymStore()
const areaStore = useAreaStore()

// ── 단계 ─────────────────────────────────────────────────────

const step = ref(1)

function buildBeforeText(content, detectedWords) {
  let result = content
  for (const word of detectedWords) {
    result = result.replaceAll(word, '')
  }
  return result
}

// ── Step 1: 그룹 관리 ─────────────────────────────────────────

const newGroupName = ref('')
const addGroupError = ref('')
const removeGroupError = ref('')
const showAddGroup = ref(false)

async function submitNewGroup() {
  const name = newGroupName.value.trim()
  if (!name) return
  addGroupError.value = ''
  try {
    await store.createGroup(name)
    newGroupName.value = ''
    showAddGroup.value = false
  } catch (e) {
    addGroupError.value = String(e)
  }
}

async function removeGroup(id) {
  if (!confirm('그룹을 삭제하면 포함된 모든 단어도 삭제됩니다. 계속하시겠습니까?')) return
  removeGroupError.value = ''
  try {
    await store.deleteGroup(id)
  } catch (e) {
    removeGroupError.value = String(e)
  }
}

// 단어 추가 (그룹별 입력)
const newWordInputs = ref({})

function getWordInput(groupId) {
  if (!newWordInputs.value[groupId]) newWordInputs.value[groupId] = ''
  return newWordInputs.value[groupId]
}

function setWordInput(groupId, val) {
  newWordInputs.value[groupId] = val
}

async function submitWord(groupId) {
  const word = (newWordInputs.value[groupId] ?? '').trim()
  if (!word) return
  try {
    await store.addWord(groupId, word)
    newWordInputs.value[groupId] = ''
  } catch (e) {
    store.error = String(e)
  }
}

// CSV 일괄 업로드 (그룹별)
const csvInputs = ref({})
const csvUploading = ref({})

function getCsvInput(groupId) {
  if (!csvInputs.value[groupId]) csvInputs.value[groupId] = ''
  return csvInputs.value[groupId]
}

function setCsvInput(groupId, val) {
  csvInputs.value[groupId] = val
}

async function submitCsv(groupId) {
  const raw = csvInputs.value[groupId] ?? ''
  const words = raw
      .split(/[\n,]+/)
      .map((w) => w.trim())
      .filter(Boolean)
  if (words.length === 0) return
  csvUploading.value[groupId] = true
  try {
    await store.addWordsBatch(groupId, words)
    csvInputs.value[groupId] = ''
  } catch (e) {
    store.error = String(e)
  } finally {
    csvUploading.value[groupId] = false
  }
}

// ── Step 2: 그룹 선택 ─────────────────────────────────────────

const selectedGroupIds = ref([])
const searching = ref(false)
const searchError = ref('')
const inspectResults = ref([])

function toggleGroup(id) {
  const idx = selectedGroupIds.value.indexOf(id)
  if (idx === -1) selectedGroupIds.value.push(id)
  else selectedGroupIds.value.splice(idx, 1)
}

function isSelected(id) {
  return selectedGroupIds.value.includes(id)
}

const selectedWordCount = computed(() => {
  const wordSet = new Set()
  for (const g of store.groups) {
    if (selectedGroupIds.value.includes(g.id)) {
      g.items.forEach((item) => {
        const w = item.word.trim()
        if (w) wordSet.add(w)
      })
    }
  }
  return wordSet.size
})

// ── Step 3: 범위 선택 ─────────────────────────────────────────

const scopeMode = ref('all')        // 'all' | 'specific'
const selectedAreaIds = ref([])

function toggleArea(id) {
  if (scopeMode.value !== 'specific') return
  const idx = selectedAreaIds.value.indexOf(id)
  if (idx === -1) selectedAreaIds.value.push(id)
  else selectedAreaIds.value.splice(idx, 1)
}

function isAreaSelected(id) {
  return selectedAreaIds.value.includes(id)
}

watch(scopeMode, () => {
  selectedAreaIds.value = []
})

// ── 위저드 네비게이션 ─────────────────────────────────────────

const canGoNext = computed(() => {
  if (step.value === 2) return selectedGroupIds.value.length > 0
  return true
})

function goPrev() {
  if (step.value === 4) backToScope()
  else if (step.value === 3) backToGroups()
  else step.value--
}

function goNext() {
  step.value++
}

// ── 검색 실행 ──────────────────────────────────────────────────

async function startSearch() {
  if (selectedGroupIds.value.length === 0) return
  if (scopeMode.value === 'specific' && selectedAreaIds.value.length === 0) return
  searching.value = true
  searchError.value = ''
  try {
    await store.fetchRecords(
        scopeMode.value === 'all' ? 'all' : 'areas',
        scopeMode.value === 'specific' ? selectedAreaIds.value : [],
    )
    inspectResults.value = performInspection(
        selectedGroupIds.value,
        store.groups,
        store.records,
    )
    step.value = 4
  } catch (e) {
    searchError.value = String(e)
  } finally {
    searching.value = false
  }
}

function backToScope() {
  if (exportResult.value) {
    exportResult.value = null
    return
  }
  inspectResults.value = []
  step.value = 3
}

function backToGroups() {
  step.value = 2
}

// ── Excel 내보내기 ─────────────────────────────────────────────

function bufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  const chunk = 8192
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(binary)
}

const exporting = ref(false)
const exportResult = ref(null)

async function exportToExcel() {
  if (inspectResults.value.length === 0) {
    alert('내보낼 검사 결과가 없습니다.')
    return
  }

  const now = new Date()
  const pad = (n) => String(n).padStart(2, '0')
  const stamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}_${pad(now.getHours())}${pad(now.getMinutes())}`
  const defaultPath = `유의어점검결과_${stamp}.xlsx`

  const filePath = await save({
    defaultPath,
    filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
  })
  if (!filePath) return

  exporting.value = true
  try {
    const wb = new Workbook()
    const ws = wb.addWorksheet('유의어점검')

    ws.columns = [
      {header: '학년', key: 'grade', width: 8},
      {header: '반', key: 'class_num', width: 8},
      {header: '번호', key: 'number', width: 8},
      {header: '이름', key: 'name', width: 10},
      {header: '영역', key: 'area', width: 16},
      {header: '활동', key: 'activity', width: 22},
      {header: '원본 내용', key: 'content', width: 60},
      {header: '탐지된 유의어', key: 'words', width: 32},
    ]

    ws.getRow(1).font = {bold: true}
    ws.getRow(1).alignment = {horizontal: 'center', vertical: 'middle'}

    for (const {record, detectedWords} of inspectResults.value) {
      const row = ws.addRow({
        grade: record.grade,
        class_num: record.class_num,
        number: record.number,
        name: record.student_name,
        area: record.area_name,
        activity: record.activity_name,
        content: record.content,
        words: detectedWords.join(', '),
      })
      row.getCell('content').alignment = {wrapText: true, vertical: 'top'}
    }

    const buffer = await wb.xlsx.writeBuffer()
    const data = bufferToBase64(buffer)
    await invoke('write_bytes_file', {path: filePath, data})
    exportResult.value = {
      fileName: filePath.split(/[\\/]/).pop(),
      filePath,
      rowCount: inspectResults.value.length,
    }
    step.value++
  } catch (e) {
    store.error = String(e)
  } finally {
    exporting.value = false
  }
}

function resetWizard() {
  step.value = 1
  selectedGroupIds.value = []
  inspectResults.value = []
  scopeMode.value = 'all'
  selectedAreaIds.value = []
  searching.value = false
  searchError.value = ''
  exporting.value = false
  exportResult.value = null
}

// ── 초기화 ────────────────────────────────────────────────────

onMounted(() => {
  store.fetchGroups()
  areaStore.fetchAreas()
})
</script>

<template>
  <div class="section">

    <!-- 헤더 -->
    <div class="toolbar">
      <div class="section-header">
        <h2 class="section-title">유의어 점검(Inspect)</h2>
        <p class="section-desc">중·고등학교 학교생활기록부 기재요령에 근거하여 유의어 및 금지어를 점검합니다.</p>
      </div>
    </div>

    <WizardLayout
        :stepCount="3"
        :currentStep="step"
        :canGoNext="canGoNext"
        :isNavigating="false"
        :showFooter="!exportResult"
        @prev="goPrev"
        @next="goNext"
    >
    <!-- 본문 -->

      <!-- ─── Step 1: 유의어 그룹 관리 ─────────────────────── -->
      <div v-if="step === 1" class="step-content">
        <div class="step-header">
          <h3 class="step-title">Step 1. 유의어 관리</h3>
          <p class="step-desc">학교생활기록부 점검에 사용할 유의어 그룹과 검색 대상 단어를 관리합니다.</p>
        </div>

        <!-- 로딩 -->
        <div v-if="store.loading" class="state-box">
          <Loader2 :size="22" class="spin"/>
          <span class="state-text">불러오는 중...</span>
        </div>

        <!-- 에러 -->
        <div v-else-if="store.error" class="error-box">{{ store.error }}</div>

        <template v-else>

          <!-- 그룹 삭제 에러 -->
          <div v-if="removeGroupError" class="error-box" style="margin-bottom: 12px;">{{ removeGroupError }}</div>

          <!-- 그룹 카드 목록 -->
          <div class="group-list">
            <div v-for="group in store.groups" :key="group.id" class="group-card">

              <!-- 그룹 헤더 -->
              <div class="group-header">
                <span class="group-name">{{ group.name }}</span>
                <span class="group-count">{{ group.items.length }}개</span>
                <button class="btn-icon btn-danger" @click="removeGroup(group.id)" title="그룹 삭제">
                  <Trash2 :size="14"/>
                </button>
              </div>

              <!-- 단어 chips -->
              <div class="word-chips">
                <span v-for="item in group.items" :key="item.id" class="chip">
                  {{ item.word }}
                  <button class="chip-del" @click="store.deleteWord(item.id)">
                    <X :size="11"/>
                  </button>
                </span>
                <span v-if="group.items.length === 0" class="chip-empty">단어 없음</span>
              </div>

              <!-- 단어 추가 입력 -->
              <div class="word-add-row">
                <input
                    class="input-sm"
                    placeholder="단어 추가..."
                    :value="getWordInput(group.id)"
                    @input="setWordInput(group.id, $event.target.value)"
                    @keydown.enter="submitWord(group.id)"
                />
                <button class="btn-sm btn-primary" @click="submitWord(group.id)">
                  <Plus :size="13"/>
                  추가
                </button>
              </div>

              <!-- 유의어 일괄 등록 -->
              <details class="csv-section">
                <summary class="csv-toggle">유의어 일괄 등록</summary>
                <div class="csv-body">
                  <textarea
                      class="csv-textarea"
                      placeholder="줄바꿈 또는 쉼표로 구분하여 여러 단어를 입력하세요"
                      :value="getCsvInput(group.id)"
                      @input="setCsvInput(group.id, $event.target.value)"
                  />
                  <button
                      class="btn-sm btn-primary"
                      :disabled="csvUploading[group.id]"
                      @click="submitCsv(group.id)"
                  >
                    <Loader2 v-if="csvUploading[group.id]" :size="13" class="spin"/>
                    <span v-else>업로드</span>
                  </button>
                </div>
              </details>

            </div>
          </div>

          <!-- 그룹 추가 -->
          <div class="add-group-area">
            <template v-if="showAddGroup">
              <div class="add-group-form">
                <input
                    v-model="newGroupName"
                    class="input-md"
                    placeholder="새 그룹 이름"
                    @keydown.enter="submitNewGroup"
                    @keydown.esc="showAddGroup = false; addGroupError = ''"
                    autofocus
                />
                <button class="btn-sm btn-primary" @click="submitNewGroup">
                  <Plus :size="13"/>
                  생성
                </button>
                <button class="btn-ghost" @click="showAddGroup = false; addGroupError = ''">취소</button>
              </div>
              <p v-if="addGroupError" class="field-error">{{ addGroupError }}</p>
            </template>
            <button v-else class="btn-add-group" @click="showAddGroup = true">
              <Plus :size="15"/>
              그룹 추가
            </button>
          </div>

        </template>
      </div>

      <!-- ─── Step 2: 그룹 선택 ─────────────────────────────── -->
      <div v-if="step === 2" class="step-content">
        <div class="step-header">
          <h3 class="step-title">Step 2. 점검할 그룹 선택</h3>
          <p class="step-desc">검색에 사용할 유의어 그룹을 하나 이상 선택하세요.</p>
        </div>

        <div class="group-select-list">
          <label
              v-for="group in store.groups"
              :key="group.id"
              class="group-select-card"
              :class="{ 'group-select-card--on': isSelected(group.id) }"
          >
            <input
                type="checkbox"
                :checked="isSelected(group.id)"
                @change="toggleGroup(group.id)"
                class="sr-only"
            />
            <div class="gsc-check">
              <Check v-if="isSelected(group.id)" :size="13"/>
            </div>
            <div class="gsc-info">
              <span class="gsc-name">{{ group.name }}</span>
              <span class="gsc-count">{{ group.items.length }}개 단어</span>
            </div>
            <ChevronRight :size="16" class="gsc-arrow"/>
          </label>
        </div>

        <div class="search-summary">
          선택된 그룹: <strong>{{ selectedGroupIds.length }}개</strong> &nbsp;|&nbsp;
          중복 제거 단어: <strong>{{ selectedWordCount }}개</strong>
        </div>
      </div>

      <!-- ─── Step 3: 점검 범위 선택 ──────────────────────────── -->
      <div v-if="step === 3" class="step-content">
        <div class="step-header">
          <h3 class="step-title">Step 3. 점검 범위 선택</h3>
          <p class="step-desc">유의어를 점검할 생기부 범위를 선택하세요.</p>
        </div>

        <!-- 모드 카드 2개: 전체 영역 / 특정 영역 -->
        <div class="scope-mode-cards">
          <div
              class="scope-mode-card"
              :class="{ 'scope-mode-card--on': scopeMode === 'all' }"
              @click="scopeMode = 'all'"
          >
            <Check v-if="scopeMode === 'all'" :size="14" class="scope-mode-check"/>
            <span class="scope-mode-title">전체 영역</span>
            <span class="scope-mode-desc">모든 생기부 기록을 점검합니다</span>
          </div>
          <div
              class="scope-mode-card"
              :class="{ 'scope-mode-card--on': scopeMode === 'specific' }"
              @click="scopeMode = 'specific'"
          >
            <Check v-if="scopeMode === 'specific'" :size="14" class="scope-mode-check"/>
            <span class="scope-mode-title">특정 영역</span>
            <span class="scope-mode-desc">점검할 영역을 직접 선택합니다</span>
          </div>
        </div>

        <!-- 개별 영역 카드 그리드 (scopeMode='specific'일 때 활성) -->
        <div class="area-section" :class="{ 'area-section--disabled': scopeMode !== 'specific' }">
          <p class="area-section-label">점검할 영역 선택</p>
          <p v-if="areaStore.areas.length === 0" class="empty-hint">등록된 영역이 없습니다.</p>
          <div v-else class="area-cards">
            <div
                v-for="area in areaStore.areas"
                :key="area.id"
                class="area-card"
                :class="{ 'area-card--selected': isAreaSelected(area.id) }"
                @click="toggleArea(area.id)"
            >
              <Check v-if="isAreaSelected(area.id)" :size="14" class="area-card-check"/>
              <span class="area-card-name">{{ area.name }}</span>
              <span class="area-card-meta">활동 {{ area.activities.length }}개</span>
            </div>
          </div>
          <p v-if="scopeMode === 'specific' && selectedAreaIds.length > 0" class="area-summary">
            {{ selectedAreaIds.length }}개 영역 선택됨
          </p>
        </div>

        <!-- 검색 에러 -->
        <div v-if="searchError" class="error-box" style="margin-bottom: 12px;">{{ searchError }}</div>

        <!-- 검색 시작 버튼 -->
        <button
            class="btn-search"
            :disabled="searching || (scopeMode === 'specific' && selectedAreaIds.length === 0)"
            @click="startSearch"
        >
          <Loader2 v-if="searching" :size="16" class="spin"/>
          <ScanSearch v-else :size="16"/>
          {{ searching ? '검색 중...' : '검색 시작' }}
        </button>
      </div>

      <!-- ─── Step 4: 결과 보고 ──────────────────────────────── -->
      <div v-if="step === 4" class="step-content">

        <!-- 점검 결과 테이블 -->
        <div class="step-header result-header">
          <div>
            <h3 class="step-title">Step 4. 점검 결과</h3>
            <p v-if="inspectResults.length > 0" class="step-desc">
              총 <strong>{{ inspectResults.length }}건</strong>의 기록에서 유의어가 탐지되었습니다.
            </p>
            <p v-else class="step-desc">
              선택한 유의어 그룹에 해당하는 기록이 없습니다.
            </p>
          </div>
          <button v-if="inspectResults.length > 0" class="btn-export" :disabled="exporting" @click="exportToExcel">
            <Loader2 v-if="exporting" :size="15" class="spin"/>
            <FileDown v-else :size="15"/>
            {{ exporting ? '저장 중...' : 'Excel 내보내기' }}
          </button>
        </div>

        <div v-if="inspectResults.length === 0" class="empty-state">
          <ScanSearch :size="40" class="empty-icon"/>
          <p class="empty-title">탐지된 유의어가 없습니다</p>
          <p class="empty-desc">유의어가 발견되지 않았습니다.</p>
        </div>

        <div v-else class="result-table-wrap">
          <table class="result-table">
            <thead>
            <tr>
              <th>학년</th>
              <th>반</th>
              <th>번호</th>
              <th>이름</th>
              <th>영역</th>
              <th>활동</th>
              <th>원본 내용</th>
              <th>발견된 유의어</th>
            </tr>
            </thead>
            <tbody>
            <tr v-for="{ record, detectedWords } in inspectResults" :key="record.id">
              <td class="cell-student">{{ record.grade || '' }}</td>
              <td class="cell-student">{{ record.class_num || '' }}</td>
              <td class="cell-student">{{ record.number || '—' }}</td>
              <td class="cell-student">{{ record.student_name || '—' }}</td>
              <td class="cell-area">{{ record.area_name || '—' }}</td>
              <td class="cell-activity">{{ record.activity_name }}</td>
              <td class="cell-content">
                <DiffView
                    :before="buildBeforeText(record.content, detectedWords)"
                    :after="record.content"
                />
              </td>
              <td class="cell-words">
                    <span
                        v-for="word in detectedWords"
                        :key="word"
                        class="word-badge"
                    >{{ word }}</span>
              </td>
            </tr>
            </tbody>
          </table>
        </div>

      </div>

      <!-- 내보내기 완료 화면 -->
      <div v-if="step === 5 && exportResult" class="result-box">
        <div class="result-check">✓</div>
        <p class="result-title">내보내기 완료</p>
        <div class="result-stats">
          <div class="stat-item">
            <span class="stat-val">{{ exportResult.rowCount }}</span>
            <span class="stat-label">행 저장됨</span>
          </div>
        </div>
        <p class="result-filename">{{ exportResult.fileName }}</p>
        <div class="result-actions">
          <button class="btn-reveal" @click="revealItemInDir(exportResult.filePath)">파일 확인</button>
          <button class="btn-reset" @click="resetWizard">처음으로 돌아가기</button>
        </div>
      </div>

    </WizardLayout>

  </div>
</template>

<style scoped>
/* ── 레이아웃 ─────────────────────────────────────────────── */
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

.toolbar-icon {
  color: #7c8db5;
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

.step-content {
}

.step-header {
  margin-bottom: 24px;
}

.step-title {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0 0 6px;
}

.step-desc {
  font-size: 15px;
  color: #7c8db5;
  margin: 0 0 24px;
}

/* ── 상태 박스 ───────────────────────────────────────────── */
.state-box {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 32px 0;
  color: #7c8db5;
  font-size: 14px;
}

.state-text {
  font-size: 14px;
}

.error-box {
  padding: 16px;
  background: rgba(220, 53, 69, 0.1);
  border: 1px solid rgba(220, 53, 69, 0.3);
  border-radius: 8px;
  color: #f87171;
  font-size: 13px;
}

/* ── 그룹 카드 (Step 1) ──────────────────────────────────── */
.group-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin-bottom: 24px;
}

.group-card {
  background: #0d1526;
  border: 1px solid #1a2035;
  border-radius: 10px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.group-name {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
  flex: 1;
}

.group-count {
  font-size: 12px;
  color: #7c8db5;
  background: #1a2035;
  padding: 2px 8px;
  border-radius: 12px;
}

.btn-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  background: transparent;
  transition: background 0.15s;
}

.btn-danger {
  color: #f87171;
}

.btn-danger:hover {
  background: rgba(248, 113, 113, 0.15);
}

/* 단어 chips */
.word-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  min-height: 28px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: #1a2035;
  border: 1px solid #2a3355;
  border-radius: 20px;
  padding: 3px 10px 3px 10px;
  font-size: 13px;
  color: #cbd5e1;
}

.chip-del {
  display: flex;
  align-items: center;
  background: none;
  border: none;
  cursor: pointer;
  color: #7c8db5;
  padding: 0;
  margin-left: 2px;
  transition: color 0.15s;
}

.chip-del:hover {
  color: #f87171;
}

.chip-empty {
  font-size: 12px;
  color: #4a5568;
  font-style: italic;
}

/* 단어 추가 행 */
.word-add-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.input-sm {
  flex: 1;
  background: #1a2035;
  border: 1px solid #2a3355;
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 13px;
  color: #e2e8f0;
  outline: none;
  transition: border-color 0.15s;
}

.input-sm:focus {
  border-color: #3b5bdb;
}

.btn-sm {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s;
}

.btn-primary {
  background: #3b5bdb;
  color: #fff;
}

.btn-primary:hover {
  background: #4c6ef5;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* CSV 섹션 */
.csv-section {
  border-top: 1px solid #1a2035;
  padding-top: 10px;
}

.csv-toggle {
  font-size: 12px;
  color: #7c8db5;
  cursor: pointer;
  user-select: none;
  list-style: none;
}

.csv-toggle::-webkit-details-marker {
  display: none;
}

.csv-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
}

.csv-textarea {
  width: 100%;
  min-height: 80px;
  background: #1a2035;
  border: 1px solid #2a3355;
  border-radius: 6px;
  padding: 8px 10px;
  font-size: 13px;
  color: #e2e8f0;
  resize: vertical;
  outline: none;
  box-sizing: border-box;
  transition: border-color 0.15s;
}

.csv-textarea:focus {
  border-color: #3b5bdb;
}

/* 그룹 추가 */
.add-group-area {
  margin-top: 4px;
}

.add-group-form {
  display: flex;
  gap: 8px;
  align-items: center;
}

.input-md {
  flex: 1;
  background: #1a2035;
  border: 1px solid #2a3355;
  border-radius: 6px;
  padding: 8px 12px;
  font-size: 14px;
  color: #e2e8f0;
  outline: none;
  transition: border-color 0.15s;
}

.input-md:focus {
  border-color: #3b5bdb;
}

.btn-ghost {
  padding: 8px 14px;
  border: 1px solid #2a3355;
  border-radius: 6px;
  background: transparent;
  color: #7c8db5;
  font-size: 14px;
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}

.btn-ghost:hover {
  border-color: #4a5568;
  color: #cbd5e1;
}

.field-error {
  font-size: 12px;
  color: #f87171;
  margin: 6px 0 0;
}

.btn-add-group {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background: transparent;
  border: 1px dashed #2a3355;
  border-radius: 8px;
  color: #7c8db5;
  font-size: 14px;
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}

.btn-add-group:hover {
  border-color: #3b5bdb;
  color: #a5b4fc;
}

/* ── Step 2: 그룹 선택 ───────────────────────────────────── */
.group-select-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
}

.group-select-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 18px;
  background: #0d1526;
  border: 1px solid #1a2035;
  border-radius: 10px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.group-select-card:hover {
  border-color: #2a3355;
  background: #111b30;
}

.group-select-card--on {
  border-color: #3b5bdb;
  background: rgba(59, 91, 219, 0.08);
}

.gsc-check {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 5px;
  border: 1.5px solid #2a3355;
  background: transparent;
  color: #52b788;
  flex-shrink: 0;
  transition: border-color 0.15s, background 0.15s;
}

.group-select-card--on .gsc-check {
  border-color: #3b5bdb;
  background: #3b5bdb;
  color: #fff;
}

.gsc-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.gsc-name {
  font-size: 14px;
  font-weight: 600;
  color: #e2e8f0;
}

.gsc-count {
  font-size: 12px;
  color: #7c8db5;
}

.gsc-arrow {
  color: #4a5568;
}

.search-summary {
  font-size: 13px;
  color: #7c8db5;
  margin-bottom: 20px;
}

.search-summary strong {
  color: #a5b4fc;
}

.btn-search {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 12px 28px;
  background: #3b5bdb;
  border: none;
  border-radius: 8px;
  color: #fff;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s;
}

.btn-search:hover:not(:disabled) {
  background: #4c6ef5;
}

.btn-search:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

/* ── Step 3: 범위 선택 ───────────────────────────────────── */
.scope-mode-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-bottom: 24px;
}

.scope-mode-card {
  position: relative;
  border: 2px solid #1a2035;
  border-radius: 10px;
  padding: 18px 20px;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 6px;
  transition: border-color 0.2s, background-color 0.2s;
}

.scope-mode-card:hover {
  border-color: rgba(59, 91, 219, 0.4);
  background-color: rgba(59, 91, 219, 0.03);
}

.scope-mode-card--on {
  border-color: rgba(59, 91, 219, 0.7);
  background-color: rgba(59, 91, 219, 0.06);
}

.scope-mode-check {
  position: absolute;
  top: 10px;
  right: 12px;
  color: #7ba8f0;
}

.scope-mode-title {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
}

.scope-mode-desc {
  font-size: 13px;
  color: var(--clr-text-subtle);
}

.area-section {
  margin-bottom: 20px;
  transition: opacity 0.2s;
}

.area-section--disabled {
  opacity: 0.35;
  pointer-events: none;
}

.area-section-label {
  font-size: 13px;
  color: #7c8db5;
  margin: 0 0 12px;
}

.area-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}

.area-card {
  position: relative;
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

.area-card-check {
  position: absolute;
  top: 10px;
  right: 12px;
  color: #7ba8f0;
}

.area-card-name {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
}

.area-card-meta {
  font-size: 13px;
  color: var(--clr-text-subtle);
}

.area-summary {
  font-size: 13px;
  color: #7c8db5;
  margin-top: 12px;
}

.empty-hint {
  font-size: 13px;
  color: #4a5568;
}

/* ── Step 4: 결과 ────────────────────────────────────────── */
.result-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
}

.btn-export {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 18px;
  background: #2d6a4f;
  border: none;
  border-radius: 8px;
  color: #d8f3dc;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: background 0.15s;
}

.btn-export:hover:not(:disabled) {
  background: #40916c;
}

.btn-export:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.result-table-wrap {
  overflow-x: auto;
  border: 1px solid #1a2035;
  border-radius: 10px;
}

.result-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.result-table thead {
  background: #0d1526;
}

.result-table th {
  padding: 10px 14px;
  text-align: left;
  font-weight: 600;
  color: #7c8db5;
  border-bottom: 1px solid #1a2035;
  white-space: nowrap;
}

.result-table td {
  padding: 10px 14px;
  vertical-align: top;
  border-bottom: 1px solid #111b30;
  color: #cbd5e1;
}

.result-table tbody tr:last-child td {
  border-bottom: none;
}

.result-table tbody tr:hover td {
  background: rgba(59, 91, 219, 0.04);
}

.col-content {
  min-width: 280px;
}

.cell-area {
  white-space: nowrap;
  color: #7c8db5;
  font-size: 12px;
}

.cell-activity {
  white-space: nowrap;
  font-weight: 500;
}

.cell-student {
  white-space: nowrap;
  font-size: 12px;
  color: #94a3b8;
}

.student-name {
  font-size: 13px;
  font-weight: 600;
  color: #cbd5e1;
}

.cell-content {
  line-height: 1.6;
  word-break: break-all;
  max-width: 400px;
}

.cell-content :deep(.diff-added) {
  background-color: rgba(251, 191, 36, 0.3);
  color: #f59e0b;
}

.cell-content :deep(.diff-removed) {
  display: none;
}

.cell-words {
  white-space: nowrap;
}

.word-badge {
  display: inline-block;
  margin: 2px 3px 2px 0;
  padding: 2px 8px;
  background: rgba(165, 180, 252, 0.12);
  border: 1px solid rgba(165, 180, 252, 0.25);
  border-radius: 12px;
  font-size: 12px;
  color: #a5b4fc;
}

/* ── Step 3: 빈 상태 ─────────────────────────────────────── */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 64px 0;
  gap: 12px;
}

.empty-icon {
  color: #2a3355;
}

.empty-title {
  font-size: 18px;
  font-weight: 600;
  color: #7c8db5;
  margin: 0;
}

.empty-desc {
  font-size: 16px;
  color: #4a5568;
  margin: 0;
}

/* ── 내보내기 완료 화면 ──────────────────────────────────── */
.result-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 64px 0;
}

.result-check {
  font-size: 48px;
  color: #34d399;
}

.result-title {
  font-size: 22px;
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
  font-size: 36px;
  font-weight: 700;
  color: #7ba8f0;
}

.stat-label {
  font-size: 13px;
  color: #7c8db5;
}

.result-filename {
  font-size: 13px;
  color: #7c8db5;
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
  transition: background 0.15s, color 0.15s;
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
  color: #7c8db5;
  font-size: 15px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.btn-reset:hover {
  background: #1a2035;
  color: #93afd4;
}

/* ── 유틸 ────────────────────────────────────────────────── */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}

.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
