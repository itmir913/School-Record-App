<script setup>
import {computed, ref} from 'vue'
import {AlertCircle, CheckCircle2, Download, FileSpreadsheet, Upload, X} from 'lucide-vue-next'
import {Workbook} from 'exceljs'
import {useStudentStore} from '../stores/student.js'
import {save} from '@tauri-apps/plugin-dialog'
import {SAMPLE_CSV} from '../data/sampleStudentCsv.ts'

const COL_ALIASES = {
  grade: ['학년', 'grade'],
  classNum: ['반', 'class', '학급', '반번호', 'classnum', 'class_num'],
  number: ['번호', 'number', 'num', '번', '출석번호'],
  name: ['이름', 'name', '성명', '학생명', '학생이름'],
}

const emit = defineEmits(['close', 'imported'])

const studentStore = useStudentStore()

const dragging = ref(false)
const fileName = ref('')
const parseError = ref('')
const importing = ref(false)
const importResult = ref(null)

const rawHeaders = ref([])
const rawData = ref([])
const colMap = ref({grade: null, classNum: null, number: null, name: null})

const fileInputRef = ref(null)

const FIELD_LABELS = {grade: '학년', classNum: '반', number: '번호', name: '이름'}

const allMapped = computed(() =>
    Object.values(colMap.value).every(v => v !== null)
)

// 전체 파싱 결과 (allMapped일 때만 계산)
const parsedRows = computed(() => {
  if (!allMapped.value || rawData.value.length === 0) return []
  const {grade: gi, classNum: ci, number: ni, name: nmi} = colMap.value
  const result = []
  rawData.value.forEach((row, i) => {
    const grade = Number(row[gi])
    const classNum = Number(row[ci])
    const number = Number(row[ni])
    const name = String(row[nmi] ?? '').trim()
    const isEmpty = (v) => v == null || String(v).trim() === ''
    if (isEmpty(row[gi]) && isEmpty(row[ci]) && isEmpty(row[ni]) && !name) return
    const errs = []
    if (!grade || isNaN(grade) || grade < 1) errs.push('학년 오류')
    if (!classNum || isNaN(classNum) || classNum < 1) errs.push('반 오류')
    if (!number || isNaN(number) || number < 1) errs.push('번호 오류')
    if (!name) errs.push('이름 없음')
    result.push({
      grade, classNum, number, name,
      _row: i + 2,
      _error: errs.length > 0 ? errs.join(', ') : null,
    })
  })
  return result
})

const validRows = computed(() => parsedRows.value.filter(r => !r._error))
const errorRows = computed(() => parsedRows.value.filter(r => r._error))

// 우측 패널용: 최대 4행, 매핑 미완료여도 raw값 표시
const livePreviewRows = computed(() => {
  if (rawData.value.length === 0) return []
  return rawData.value.slice(0, 4).map((row, i) => ({
    _row: i + 2,
    grade: colMap.value.grade !== null ? row[colMap.value.grade] : null,
    classNum: colMap.value.classNum !== null ? row[colMap.value.classNum] : null,
    number: colMap.value.number !== null ? row[colMap.value.number] : null,
    name: colMap.value.name !== null ? String(row[colMap.value.name] ?? '') : null,
  }))
})

async function downloadSample() {
  const path = await save({
    title: '샘플 파일 저장',
    defaultPath: '예시_학생_명렬표.csv',
    filters: [{name: 'CSV', extensions: ['csv']}],
  })
  if (!path) return
  try {
    await studentStore.writeSampleFile(path, '\uFEFF' + SAMPLE_CSV)
  } catch (e) {
    parseError.value = '샘플 파일 저장 실패: ' + String(e)
  }
}

function onDragOver(e) {
  e.preventDefault()
  dragging.value = true
}

function onDragLeave() {
  dragging.value = false
}

function onDrop(e) {
  e.preventDefault()
  dragging.value = false
  const file = e.dataTransfer.files[0]
  if (file) processFile(file)
}

function onFileChange(e) {
  const file = e.target.files[0]
  if (file) processFile(file)
  e.target.value = ''
}

function parseCsv(text) {
  const rows = []
  const lines = text.replace(/\r\n/g, '\n').replace(/\r/g, '\n').split('\n')
  for (const line of lines) {
    if (!line.trim()) continue
    const row = []
    let field = ''
    let inQuotes = false
    for (let i = 0; i < line.length; i++) {
      const ch = line[i]
      if (inQuotes) {
        if (ch === '"' && line[i + 1] === '"') { field += '"'; i++ }
        else if (ch === '"') inQuotes = false
        else field += ch
      } else {
        if (ch === '"') inQuotes = true
        else if (ch === ',') { row.push(field); field = '' }
        else field += ch
      }
    }
    row.push(field)
    rows.push(row)
  }
  return rows
}

function cellValue(v) {
  if (v === null || v === undefined) return ''
  if (typeof v === 'object') {
    if (v.richText) return v.richText.map(r => r.text).join('')
    if (v.text !== undefined) return String(v.text)
    if (v instanceof Date) return v.toLocaleDateString()
  }
  return String(v)
}

function processFile(file) {
  fileName.value = file.name
  parseError.value = ''
  importResult.value = null
  rawHeaders.value = []
  rawData.value = []

  const ext = file.name.split('.').pop().toLowerCase()
  if (!['csv', 'xlsx', 'xls'].includes(ext)) {
    parseError.value = 'CSV(.csv) 또는 엑셀(.xlsx, .xls) 파일만 지원합니다.'
    return
  }
  if (ext === 'xls') {
    parseError.value = 'XLS 형식은 지원되지 않습니다. XLSX 형식으로 변환 후 다시 시도해주세요.'
    return
  }

  const reader = new FileReader()
  reader.onload = async (ev) => {
    try {
      let rows
      if (ext === 'csv') {
        rows = parseCsv(new TextDecoder('utf-8').decode(new Uint8Array(ev.target.result)))
      } else {
        const workbook = new Workbook()
        await workbook.xlsx.load(ev.target.result)
        const worksheet = workbook.worksheets[0]
        rows = []
        worksheet.eachRow((row) => {
          rows.push(row.values.slice(1).map(cellValue))
        })
        if (rows.length > 1) {
          const headerLen = rows[0].length
          for (let i = 1; i < rows.length; i++) {
            while (rows[i].length < headerLen) rows[i].push('')
          }
        }
      }

      if (rows.length < 2) {
        parseError.value = '데이터가 없습니다. 헤더 행 포함 최소 2행이 필요합니다.'
        return
      }

      rawHeaders.value = rows[0].map(h => String(h ?? '').trim())
      rawData.value = rows.slice(1)
      autoDetectColumns()
    } catch (err) {
      parseError.value = '파일을 파싱하는 중 오류가 발생했습니다: ' + err.message
    }
  }
  reader.readAsArrayBuffer(file)
}

function resetFile() {
  fileName.value = ''
  rawHeaders.value = []
  rawData.value = []
  colMap.value = {grade: null, classNum: null, number: null, name: null}
  parseError.value = ''
  importResult.value = null
}

function autoDetectColumns() {
  const map = {grade: null, classNum: null, number: null, name: null}
  rawHeaders.value.forEach((header, idx) => {
    const h = header.toLowerCase().replace(/\s/g, '')
    for (const [field, aliases] of Object.entries(COL_ALIASES)) {
      if (map[field] === null && aliases.includes(h)) {
        map[field] = idx
      }
    }
  })
  colMap.value = map
}

async function doImport() {
  const rows = validRows.value
  if (rows.length === 0) return

  importing.value = true
  try {
    const students = rows.map(r => ({
      grade: r.grade,
      class_num: r.classNum,
      number: r.number,
      name: r.name,
    }))
    const {inserted, updated} = await studentStore.bulkUpsertStudents(students)
    importResult.value = {inserted, updated, total: rows.length}
    emit('imported')
  } catch (e) {
    parseError.value = '가져오기 실패: ' + String(e)
  } finally {
    importing.value = false
  }
}
</script>

<template>
  <div class="modal-overlay">
    <div class="modal modal-container">

      <!-- 항상 DOM에 존재하는 숨김 파일 입력 -->
      <input
          ref="fileInputRef"
          type="file"
          accept=".csv,.xlsx,.xls"
          style="display:none"
          @change="onFileChange"
      />

      <!-- 헤더 -->
      <div class="modal-hdr">
        <h2 class="modal-title">학생 일괄 추가</h2>
        <button class="modal-close" @click="emit('close')">
          <X :size="18"/>
        </button>
      </div>

      <!-- 바디 -->
      <div class="modal-body">

        <!-- 샘플 다운로드 (항상 표시) -->
        <div class="sample-section">
          <div class="guide-text">
            <p>학생 명단이 담긴 CSV 또는 엑셀 파일을 업로드해 주세요.</p>
            <p>파일 내에 '학년, 반, 번호, 이름' 열이 포함되어 있는지 확인해 주세요.</p>
          </div>
          <button class="btn-sample" @click="downloadSample">
            <Download :size="14"/>
            샘플 파일 다운로드
          </button>
        </div>

        <!-- 파일 미선택: 드롭존 -->
        <div
            v-if="!rawHeaders.length"
            class="drop-zone"
            :class="dragging ? 'drop-zone--active' : ''"
            @dragover="onDragOver"
            @dragleave="onDragLeave"
            @drop="onDrop"
            @click="fileInputRef.click()"
        >
          <FileSpreadsheet :size="32" class="drop-icon"/>
          <p class="drop-text">
            파일을 여기에 드래그하거나 <span class="drop-link">파일 선택</span>
          </p>
          <p class="drop-hint">CSV, XLSX, XLS 지원</p>
        </div>

        <!-- 파일 선택됨: 2단 레이아웃 -->
        <div v-else class="split-layout">

          <!-- 좌: 열 매핑 -->
          <div class="mapping-column">
            <div class="mapping-panel">
              <div class="mapping-panel-head">
                <p class="mapping-title">열 매핑 확인</p>
                <button class="btn-change-file" @click="resetFile();">
                  <FileSpreadsheet :size="13"/>
                  파일 초기화
                </button>
              </div>
              <p class="mapping-desc">📄 {{ fileName }}</p>
              <div class="mapping-grid">
                <div
                    v-for="(label, field) in FIELD_LABELS"
                    :key="field"
                    class="mapping-row"
                >
                  <span class="mapping-label">{{ label }}</span>
                  <select
                      class="mapping-select"
                      :class="colMap[field] === null ? 'mapping-select--empty' : 'mapping-select--set'"
                      :value="colMap[field] ?? ''"
                      @change="colMap[field] = $event.target.value === '' ? null : Number($event.target.value)"
                  >
                    <option value="">— 선택 안 됨 —</option>
                    <option
                        v-for="(header, idx) in rawHeaders"
                        :key="idx"
                        :value="idx"
                    >{{ header || `(${idx + 1}번째 열)` }}
                    </option>
                  </select>
                  <span v-if="colMap[field] !== null" class="mapping-badge mapping-badge--ok">자동</span>
                  <span v-else class="mapping-badge mapping-badge--empty">미선택</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 우: 실시간 미리보기 -->
          <div class="preview-column">
            <div class="preview-panel">
              <p class="preview-panel-title">미리보기</p>
              <p class="preview-desc">
                <template v-if="allMapped">
                  총 {{ parsedRows.length }}행
                  <span v-if="errorRows.length > 0" class="error-count">(오류 {{ errorRows.length }}행 제외)</span>
                </template>
                <template v-else>열 매핑을 완료하면 전체 인원이 표시됩니다.</template>
              </p>
              <div class="preview-wrap">
                <table class="preview-table">
                  <thead>
                  <tr>
                    <th>행</th>
                    <th>학년</th>
                    <th>반</th>
                    <th>번호</th>
                    <th>이름</th>
                  </tr>
                  </thead>
                  <tbody>
                  <tr v-for="row in livePreviewRows" :key="row._row">
                    <td class="td-row">{{ row._row }}</td>
                    <td :class="row.grade === null ? 'td-unmapped' : ''">{{ row.grade ?? '—' }}</td>
                    <td :class="row.classNum === null ? 'td-unmapped' : ''">{{ row.classNum ?? '—' }}</td>
                    <td :class="row.number === null ? 'td-unmapped' : ''">{{ row.number ?? '—' }}</td>
                    <td :class="row.name === null ? 'td-unmapped' : ''">{{ row.name ?? '—' }}</td>
                  </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>

        <!-- 오류 -->
        <div v-if="parseError" class="msg-error alert">
          <AlertCircle :size="15"/>
          {{ parseError }}
        </div>

        <!-- 가져오기 결과 -->
        <div v-if="importResult" class="msg-success alert">
          <CheckCircle2 :size="15"/>
          <template v-if="importResult.inserted > 0 && importResult.updated > 0">
            {{ importResult.inserted }}명 추가, {{ importResult.updated }}명 업데이트됨.
          </template>
          <template v-else-if="importResult.inserted > 0">
            {{ importResult.inserted }}명 추가됨.
          </template>
          <template v-else>
            {{ importResult.updated }}명 업데이트됨.
          </template>
        </div>

      </div>

      <!-- 푸터 -->
      <div class="modal-ftr modal-footer">
        <button class="btn-secondary" @click="emit('close')">닫기</button>
        <button
            class="btn-primary btn-import"
            :disabled="!allMapped || validRows.length === 0 || importing || !!importResult"
            @click="doImport"
        >
          <Upload :size="15"/>
          {{ importing ? '가져오는 중...' : `${allMapped ? validRows.length : 0}명 가져오기` }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal {
  max-width: 860px;
  max-height: 85vh;
  overflow: hidden;
}

.modal-title {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0;
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 20px 24px 8px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}


/* 샘플 섹션 */
.sample-section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.guide-text {
  font-size: 15px;
  color: #7ba3d4;
  margin: 0;
  line-height: 1.6;
}

.btn-sample {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 12px;
  border-radius: 8px;
  border: 1px solid rgba(59, 91, 219, 0.3);
  background: rgba(59, 91, 219, 0.08);
  color: #7ba8f0;
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
  transition: background-color 0.15s;
  flex-shrink: 0;
}

.btn-sample:hover {
  background: rgba(59, 91, 219, 0.15);
}

/* 드롭존 */
.drop-zone {
  border: 2px dashed #1a2035;
  border-radius: 14px;
  padding: 48px 24px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.drop-zone:hover,
.drop-zone--active {
  border-color: rgba(59, 91, 219, 0.5);
  background-color: rgba(59, 91, 219, 0.04);
}

.drop-icon {
  color: var(--clr-text-hint);
}

.drop-text {
  font-size: 15px;
  color: #7ba3d4;
  margin: 0;
}

.drop-link {
  color: #7ba8f0;
  text-decoration: underline;
}

.drop-hint {
  font-size: 13px;
  color: var(--clr-text-hint);
  margin: 0;
}

/* 2단 레이아웃 */
.split-layout {
  display: flex;
  gap: 16px;
  align-items: stretch;
}

.mapping-column {
  width: 300px;
  flex-shrink: 0;
}

.preview-column {
  flex: 1;
  min-width: 0;
}

/* 열 매핑 패널 */
.mapping-panel {
  height: 100%;
  background-color: rgba(59, 91, 219, 0.05);
  border: 1px solid rgba(59, 91, 219, 0.2);
  border-radius: 12px;
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.mapping-panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.mapping-title {
  font-size: 14px;
  font-weight: 600;
  color: #93afd4;
  margin: 0;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.mapping-desc {
  font-size: 14px;
  color: var(--clr-text-subtle);
  margin: -6px 0 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mapping-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.mapping-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mapping-label {
  font-size: 15px;
  font-weight: 600;
  color: #c8d8f0;
  width: 36px;
  flex-shrink: 0;
}

.mapping-select {
  flex: 1;
  min-width: 0;
  padding: 6px 8px;
  border-radius: 8px;
  border: 1px solid #1a2035;
  background-color: #080b14;
  color: #e2e8f0;
  font-size: 13px;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s;
}

.mapping-select:focus {
  border-color: rgba(59, 91, 219, 0.5);
}

.mapping-select--set {
  border-color: rgba(52, 211, 153, 0.3);
}

.mapping-select--empty {
  border-color: rgba(251, 191, 36, 0.3);
}

.mapping-badge {
  font-size: 11px;
  font-weight: 600;
  border-radius: 4px;
  padding: 2px 5px;
  flex-shrink: 0;
  white-space: nowrap;
}

.mapping-badge--ok {
  color: #34d399;
  background-color: rgba(52, 211, 153, 0.1);
}

.mapping-badge--empty {
  color: #fbbf24;
  background-color: rgba(251, 191, 36, 0.1);
}

.btn-change-file {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 10px;
  border-radius: 7px;
  border: 1px solid rgba(59, 91, 219, 0.3);
  background: rgba(59, 91, 219, 0.08);
  color: #7ba8f0;
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
  transition: background-color 0.15s;
}

.btn-change-file:hover {
  background: rgba(59, 91, 219, 0.15);
}

/* 미리보기 패널 */
.preview-panel {
  flex: 1;
  height: 100%;
  background-color: rgba(59, 91, 219, 0.05);
  border: 1px solid rgba(59, 91, 219, 0.2);
  border-radius: 12px;
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.preview-panel-title {
  font-size: 14px;
  font-weight: 600;
  color: #93afd4;
  margin: 0;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.preview-desc {
  font-size: 14px;
  color: var(--clr-text-subtle);
  margin: -6px 0 0;
}

.error-count {
  color: #fca5a5;
}

.preview-wrap {
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
}

.preview-table {
  width: 100%;
  border-collapse: collapse;
}

.preview-table th {
  font-size: 12px;
  font-weight: 600;
  color: var(--clr-text-subtle);
  padding: 8px 10px;
  background-color: #080b14;
  border-bottom: 1px solid #1a2035;
  text-align: left;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.preview-table td {
  font-size: 14px;
  color: #c8d8f0;
  padding: 7px 10px;
  border-bottom: 1px solid rgba(26, 32, 53, 0.6);
}

.preview-table tr:last-child td {
  border-bottom: none;
}

.td-row {
  color: var(--clr-text-subtle);
  font-size: 13px;
}

.td-unmapped {
  color: var(--clr-text-subtle);
}

/* 알림 — layout override (색상은 전역 .msg-error / .msg-success 사용) */
.alert {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-radius: 10px;
}

/* 푸터 */
.modal-footer {
  justify-content: flex-end;
  padding-bottom: 20px;
  gap: 8px;
}

.btn-import {
  display: flex;
  align-items: center;
  gap: 7px;
}

.btn-import:disabled {
  opacity: 0.4;
  box-shadow: none;
}
</style>
