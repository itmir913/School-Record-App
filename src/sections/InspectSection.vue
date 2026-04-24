<script setup>
import {computed, onMounted, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'
import {Workbook} from 'exceljs'
import {
  ArrowLeft,
  ArrowRight,
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
import {performInspection} from '../services/synonymService'

const store = useSynonymStore()

// ── 단계 ─────────────────────────────────────────────────────

const step = ref(1)
const bodyRef = ref(null)

watch(step, () => {
  bodyRef.value?.scrollTo({top: 0, behavior: 'smooth'})
})

// ── Step 1: 그룹 관리 ─────────────────────────────────────────

const newGroupName = ref('')
const addGroupError = ref('')
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
  await store.deleteGroup(id)
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
  await store.addWord(groupId, word)
  newWordInputs.value[groupId] = ''
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
    for (const word of words) {
      await store.addWord(groupId, word)
    }
    csvInputs.value[groupId] = ''
  } finally {
    csvUploading.value[groupId] = false
  }
}

// ── Step 2: 그룹 선택 ─────────────────────────────────────────

const selectedGroupIds = ref([])
const searching = ref(false)
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

async function startSearch() {
  if (selectedGroupIds.value.length === 0) return
  searching.value = true
  try {
    await store.fetchRecords()
    inspectResults.value = performInspection(
        selectedGroupIds.value,
        store.groups,
        store.records,
    )
    step.value = 3
  } finally {
    searching.value = false
  }
}

function backToSelect() {
  inspectResults.value = []
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
        name: record.name,
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
  } finally {
    exporting.value = false
  }
}

// ── 초기화 ────────────────────────────────────────────────────

onMounted(() => {
  store.fetchGroups()
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
      <div class="step-indicator">
        <div v-for="n in 3" :key="n" class="step-dot"
             :class="{ 'step-dot--active': step === n, 'step-dot--done': step > n }">
          <Check v-if="step > n" :size="13"/>
          <span v-else>{{ n }}</span>
        </div>
      </div>
    </div>

    <!-- 본문 -->
    <div class="wizard-body" ref="bodyRef">

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

              <!-- CSV 일괄 업로드 -->
              <details class="csv-section">
                <summary class="csv-toggle">CSV 일괄 업로드</summary>
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
                <button class="btn-sm btn-primary" @click="submitNewGroup"><Plus :size="13"/>생성</button>
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

        <div v-if="selectedGroupIds.length > 0" class="search-summary">
          선택된 그룹: <strong>{{ selectedGroupIds.length }}개</strong> &nbsp;|&nbsp;
          중복 제거 단어: <strong>{{ selectedWordCount }}개</strong>
        </div>

        <button
            class="btn-search"
            :disabled="selectedGroupIds.length === 0 || searching"
            @click="startSearch"
        >
          <Loader2 v-if="searching" :size="16" class="spin"/>
          <ScanSearch v-else :size="16"/>
          {{ searching ? '검색 중...' : '검색 시작' }}
        </button>
      </div>

      <!-- ─── Step 3: 결과 보고 ──────────────────────────────── -->
      <div v-if="step === 3" class="step-content">
        <div class="step-header result-header">
          <div>
            <h3 class="step-title">Step 3. 점검 결과</h3>
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
              <th>영역</th>
              <th>활동</th>
              <th>학생</th>
              <th class="col-content">원본 내용</th>
              <th>탐지된 유의어</th>
            </tr>
            </thead>
            <tbody>
            <tr v-for="{ record, detectedWords } in inspectResults" :key="record.id">
              <td class="cell-area">{{ record.area_name || '—' }}</td>
              <td class="cell-activity">{{ record.activity_name }}</td>
              <td class="cell-student">
                {{ record.grade }}-{{ record.class_num }}-{{ record.number }}<br/>
                <span class="student-name">{{ record.student_name }}</span>
              </td>
              <td class="cell-content">{{ record.content }}</td>
              <td class="cell-words">
                  <span
                      v-for="(word, i) in detectedWords"
                      :key="i"
                      class="word-badge"
                  >{{ word }}</span>
              </td>
            </tr>
            </tbody>
          </table>
        </div>
      </div>

    </div>

    <!-- 하단 네비게이션 -->
    <div class="wizard-footer">
      <button
          class="btn-prev"
          :disabled="step === 1"
          @click="step === 3 ? backToSelect() : step--"
      >
        <ArrowLeft :size="15"/>
        이전
      </button>
      <button
          v-if="step < 2"
          class="btn-next"
          @click="step++"
      >
        다음
        <ArrowRight :size="15"/>
      </button>
    </div>

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

.wizard-body {
  flex: 1;
  overflow-y: auto;
  padding: 32px 40px 48px;
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

/* ── 단계 표시기 ─────────────────────────────────────────── */
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

/* ── Step 3: 결과 ────────────────────────────────────────── */
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

/* ── 하단 네비게이션 ─────────────────────────────────────── */
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
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 9px 18px;
  border-radius: 7px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  border: 1px solid #2a3355;
  background: transparent;
  color: #7c8db5;
}

.btn-prev:hover:not(:disabled),
.btn-next:hover:not(:disabled) {
  background: #1a2035;
  color: #cbd5e1;
}

.btn-prev:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.btn-next {
  border-color: #3b5bdb;
  color: #a5b4fc;
}

.btn-next:hover:not(:disabled) {
  background: rgba(59, 91, 219, 0.15);
  color: #c7d2fe;
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
