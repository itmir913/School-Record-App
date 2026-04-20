<script setup>
import {computed, ref} from 'vue'
import {AlertCircle, CheckCircle2, Download, FileSpreadsheet, Upload, X} from 'lucide-vue-next'
import * as XLSX from 'xlsx'
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'

const SAMPLE_CSV = `학년,반,번호,이름
3,1,1,홍길동(예시)
3,1,2,김철수(예시)
3,2,1,이영희(예시)
`

const COL_ALIASES = {
  grade: ['학년', 'grade'],
  classNum: ['반', 'class', '학급', '반번호', 'classnum', 'class_num'],
  number: ['번호', 'number', 'num', '번', '출석번호', '순번'],
  name: ['이름', 'name', '성명', '학생명', '학생이름'],
}

const emit = defineEmits(['close', 'imported'])

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
    if (!grade && !classNum && !number && !name) return
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
    defaultPath: 'sample_students.csv',
    filters: [{name: 'CSV', extensions: ['csv']}],
  })
  if (!path) return
  await invoke('write_text_file', {path, content: '\uFEFF' + SAMPLE_CSV})
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

  const reader = new FileReader()
  reader.onload = (ev) => {
    try {
      const data = new Uint8Array(ev.target.result)
      const wb = XLSX.read(data, {type: 'array'})
      const ws = wb.Sheets[wb.SheetNames[0]]
      const rows = XLSX.utils.sheet_to_json(ws, {header: 1, defval: ''})

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
    const {inserted, updated} = await invoke('bulk_upsert_students', {students})
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
  <div class="overlay">
    <div class="modal">

      <!-- 항상 DOM에 존재하는 숨김 파일 입력 -->
      <input
          ref="fileInputRef"
          type="file"
          accept=".csv,.xlsx,.xls"
          style="display:none"
          @change="onFileChange"
      />

      <!-- 헤더 -->
      <div class="modal-header">
        <h2 class="modal-title">학생 일괄 추가</h2>
        <button class="close-btn" @click="emit('close')">
          <X :size="18"/>
        </button>
      </div>

      <!-- 바디 -->
      <div class="modal-body">

        <!-- 샘플 다운로드 (항상 표시) -->
        <div class="sample-section">
          <p class="guide-text">
            학생 명단 CSV 또는 엑셀 파일을 업로드하세요. 열 순서는 자유롭게 작성해도 됩니다.
          </p>
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
                <button class="btn-change-file" @click="resetFile(); fileInputRef.click()">
                  <FileSpreadsheet :size="13"/>
                  파일 변경
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
        <div v-if="parseError" class="alert alert--error">
          <AlertCircle :size="15"/>
          {{ parseError }}
        </div>

        <!-- 가져오기 결과 -->
        <div v-if="importResult" class="alert alert--success">
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
      <div class="modal-footer">
        <button class="btn-cancel" @click="emit('close')">닫기</button>
        <button
            class="btn-import"
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
.overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(4, 6, 12, 0.75);
  backdrop-filter: blur(6px);
}

.modal {
  width: 100%;
  max-width: 860px;
  max-height: 85vh;
  background-color: #0e1220;
  border: 1px solid #1a2035;
  border-radius: 20px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.7);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 0;
  flex-shrink: 0;
}

.modal-title {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: none;
  border: none;
  color: #5a7aaa;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.close-btn:hover {
  background-color: #1a2035;
  color: #93afd4;
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

.modal-body::-webkit-scrollbar {
  width: 4px;
}

.modal-body::-webkit-scrollbar-thumb {
  background-color: #1a2035;
  border-radius: 2px;
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
  color: #3d5580;
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
  color: #3d5580;
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
  color: #5a7aaa;
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
  color: #5a7aaa;
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
  color: #5a7aaa;
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
  color: #5a7aaa;
  font-size: 13px;
}

.td-unmapped {
  color: #2a3a58;
}

/* 알림 */
.alert {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-radius: 10px;
  font-size: 15px;
}

.alert--error {
  background-color: rgba(239, 68, 68, 0.07);
  border: 1px solid rgba(239, 68, 68, 0.25);
  color: #fca5a5;
}

.alert--success {
  background-color: rgba(52, 211, 153, 0.07);
  border: 1px solid rgba(52, 211, 153, 0.25);
  color: #6ee7b7;
}

.skip-hint {
  color: #5a7aaa;
  font-size: 14px;
}

/* 푸터 */
.modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 24px 20px;
  border-top: 1px solid #1a2035;
  flex-shrink: 0;
}

.btn-cancel {
  padding: 10px 20px;
  border-radius: 10px;
  border: 1px solid #1a2035;
  background: none;
  color: #7ba3d4;
  font-size: 16px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-cancel:hover {
  background-color: #1a2035;
}

.btn-import {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 10px 24px;
  border-radius: 10px;
  border: none;
  background-color: #3b5bdb;
  color: white;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s;
  box-shadow: 0 4px 16px rgba(59, 91, 219, 0.2);
}

.btn-import:hover:not(:disabled) {
  background-color: #4c6ef5;
}

.btn-import:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  box-shadow: none;
}
</style>
