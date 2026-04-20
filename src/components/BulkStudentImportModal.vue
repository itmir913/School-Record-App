<script setup>
import {ref} from 'vue'
import {AlertCircle, CheckCircle2, Download, FileSpreadsheet, Upload, X} from 'lucide-vue-next'
import * as XLSX from 'xlsx'
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'
// 샘플 CSV 내용 (src/assets/sample_students.csv 와 동일한 구조)
const SAMPLE_CSV = `학년,반,번호,이름
3,1,1,홍길동(예시)
3,1,2,김철수(예시)
3,2,1,이영희(예시)
`

const emit = defineEmits(['close', 'imported'])

// 파일 드래그 상태
const dragging = ref(false)
// 파싱된 행 목록: {grade, classNum, number, name, _error?}
const parsedRows = ref([])
const parseError = ref('')
const fileName = ref('')
const importing = ref(false)
const importResult = ref(null) // {inserted, total}

const fileInputRef = ref(null)

// 샘플 CSV 다운로드 (OS 저장 다이얼로그)
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
  parsedRows.value = []
  importResult.value = null

  const ext = file.name.split('.').pop().toLowerCase()
  if (!['csv', 'xlsx', 'xls'].includes(ext)) {
    parseError.value = 'CSV(.csv) 또는 엑셀(.xlsx, .xls) 파일만 지원합니다.'
    return
  }

  const reader = new FileReader()
  reader.onload = (e) => {
    try {
      const data = new Uint8Array(e.target.result)
      const wb = XLSX.read(data, {type: 'array'})
      const ws = wb.Sheets[wb.SheetNames[0]]
      const rows = XLSX.utils.sheet_to_json(ws, {header: 1, defval: ''})
      parseRows(rows)
    } catch (err) {
      parseError.value = '파일을 파싱하는 중 오류가 발생했습니다: ' + err.message
    }
  }
  reader.readAsArrayBuffer(file)
}

function parseRows(rows) {
  if (rows.length < 2) {
    parseError.value = '데이터가 없습니다. 헤더 행 포함 최소 2행이 필요합니다.'
    return
  }

  // 헤더 행 건너뜀 (첫 번째 행)
  const parsed = []
  for (let i = 1; i < rows.length; i++) {
    const row = rows[i]
    const grade = Number(row[0])
    const classNum = Number(row[1])
    const number = Number(row[2])
    const name = String(row[3] ?? '').trim()

    const rowErrors = []
    if (!grade || isNaN(grade) || grade < 1) rowErrors.push('학년 오류')
    if (!classNum || isNaN(classNum) || classNum < 1) rowErrors.push('반 오류')
    if (!number || isNaN(number) || number < 1) rowErrors.push('번호 오류')
    if (!name) rowErrors.push('이름 없음')

    parsed.push({
      grade, classNum, number, name,
      _row: i + 1,
      _error: rowErrors.length > 0 ? rowErrors.join(', ') : null,
    })
  }

  // 완전히 빈 행 제거
  const filtered = parsed.filter(r => !(r.grade === 0 && r.classNum === 0 && r.number === 0 && !r.name))
  parsedRows.value = filtered
}

const validRows = () => parsedRows.value.filter(r => !r._error)
const errorRows = () => parsedRows.value.filter(r => r._error)

async function doImport() {
  const rows = validRows()
  if (rows.length === 0) return

  importing.value = true
  try {
    const students = rows.map(r => ({
      grade: r.grade,
      classNum: r.classNum,
      number: r.number,
      name: r.name,
    }))
    const inserted = await invoke('bulk_upsert_students', {students})
    importResult.value = {inserted, total: rows.length}
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

      <!-- 헤더 -->
      <div class="modal-header">
        <h2 class="modal-title">학생 일괄 추가</h2>
        <button class="close-btn" @click="emit('close')">
          <X :size="18"/>
        </button>
      </div>

      <!-- 바디 -->
      <div class="modal-body">

        <!-- 샘플 다운로드 -->
        <div class="sample-section">
          <p class="guide-text">
            <strong>학년, 반, 번호, 이름</strong> 열 순서로 작성된 CSV 또는 엑셀 파일을 업로드하세요.
          </p>
          <button class="btn-sample" @click="downloadSample">
            <Download :size="14"/>
            샘플 파일 다운로드
          </button>
        </div>

        <!-- 업로드 영역 -->
        <div
            class="drop-zone"
            :class="dragging ? 'drop-zone--active' : ''"
            @dragover="onDragOver"
            @dragleave="onDragLeave"
            @drop="onDrop"
            @click="fileInputRef.click()"
        >
          <input
              ref="fileInputRef"
              type="file"
              accept=".csv,.xlsx,.xls"
              style="display:none"
              @change="onFileChange"
          />
          <FileSpreadsheet :size="32" class="drop-icon"/>
          <p class="drop-text">
            파일을 여기에 드래그하거나 <span class="drop-link">파일 선택</span>
          </p>
          <p class="drop-hint">CSV, XLSX, XLS 지원</p>
          <p v-if="fileName" class="drop-filename">{{ fileName }}</p>
        </div>

        <!-- 파싱 오류 -->
        <div v-if="parseError" class="alert alert--error">
          <AlertCircle :size="15"/>
          {{ parseError }}
        </div>

        <!-- 가져오기 결과 -->
        <div v-if="importResult" class="alert alert--success">
          <CheckCircle2 :size="15"/>
          {{ importResult.total }}명 중 {{ importResult.inserted }}명 추가됨
          <span v-if="importResult.total - importResult.inserted > 0" class="skip-hint">
            ({{ importResult.total - importResult.inserted }}명은 이미 존재하여 건너뜀)
          </span>
        </div>

        <!-- 미리보기 -->
        <template v-if="parsedRows.length > 0 && !importResult">
          <div class="preview-header">
            <span class="preview-count">
              총 {{ parsedRows.length }}행
              <span v-if="errorRows().length > 0" class="error-count">
                (오류 {{ errorRows().length }}행 제외)
              </span>
            </span>
          </div>
          <div class="preview-wrap">
            <table class="preview-table">
              <thead>
              <tr>
                <th>행</th>
                <th>학년</th>
                <th>반</th>
                <th>번호</th>
                <th>이름</th>
                <th>상태</th>
              </tr>
              </thead>
              <tbody>
              <tr
                  v-for="row in parsedRows"
                  :key="row._row"
                  :class="row._error ? 'row--error' : ''"
              >
                <td class="td-row">{{ row._row }}</td>
                <td>{{ row.grade || '-' }}</td>
                <td>{{ row.classNum || '-' }}</td>
                <td>{{ row.number || '-' }}</td>
                <td>{{ row.name || '-' }}</td>
                <td class="td-status">
                  <span v-if="row._error" class="status-error">{{ row._error }}</span>
                  <span v-else class="status-ok">✓</span>
                </td>
              </tr>
              </tbody>
            </table>
          </div>
        </template>
      </div>

      <!-- 푸터 -->
      <div class="modal-footer">
        <button class="btn-cancel" @click="emit('close')">닫기</button>
        <button
            class="btn-import"
            :disabled="validRows().length === 0 || importing || !!importResult"
            @click="doImport"
        >
          <Upload :size="15"/>
          {{ importing ? '가져오는 중...' : `${validRows().length}명 가져오기` }}
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
  max-width: 580px;
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

.guide-text strong {
  color: #c8d8f0;
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
  padding: 32px 24px;
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

.drop-filename {
  font-size: 14px;
  color: #93c5fd;
  margin: 4px 0 0;
  font-weight: 500;
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

/* 미리보기 */
.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.preview-count {
  font-size: 14px;
  color: #7ba3d4;
}

.error-count {
  color: #fca5a5;
}

.preview-wrap {
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
  max-height: 240px;
  overflow-y: auto;
}

.preview-wrap::-webkit-scrollbar {
  width: 4px;
}

.preview-wrap::-webkit-scrollbar-thumb {
  background-color: #1a2035;
  border-radius: 2px;
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

.row--error td {
  color: #7ba3d4;
  background-color: rgba(239, 68, 68, 0.04);
}

.td-row {
  color: #5a7aaa;
  font-size: 13px;
}

.td-status {
  text-align: center;
}

.status-ok {
  color: #34d399;
  font-size: 13px;
}

.status-error {
  font-size: 12px;
  color: #f87171;
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
