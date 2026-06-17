<script setup>
import {computed, ref} from 'vue'
import {AlertCircle, CheckCircle2, Download, FileSpreadsheet, Upload, X} from 'lucide-vue-next'
import {Workbook} from 'exceljs'
import {useStudentStore} from '../stores/student.js'
import {useFileStore} from '../stores/file.js'
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
const fileStore = useFileStore()

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

function bufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  const chunk = 8192
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(binary)
}

async function downloadSample() {
  const path = await save({
    title: '샘플 파일 저장',
    defaultPath: '예시_학생_명렬표.xlsx',
    filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
  })

  if (!path) return

  try {
    const csvRows = parseCsv(SAMPLE_CSV)
    const workbook = new Workbook()
    const worksheet = workbook.addWorksheet('예시')
    for (const row of csvRows) {
      worksheet.addRow(row)
    }
    const buffer = await workbook.xlsx.writeBuffer()
    const data = bufferToBase64(buffer)
    await fileStore.writeBytesFile(path, data)
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

function decodeCSVBytes(buffer) {
  const bytes = new Uint8Array(buffer)
  if (bytes[0] === 0xEF && bytes[1] === 0xBB && bytes[2] === 0xBF)
    return new TextDecoder('utf-8').decode(bytes.subarray(3))
  for (const enc of ['utf-8', 'euc-kr'])
    try { return new TextDecoder(enc, { fatal: true }).decode(bytes) } catch {}
  return new TextDecoder('utf-8').decode(bytes)
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
        rows = parseCsv(decodeCSVBytes(ev.target.result))
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
    <div class="modal-container max-w-[860px] max-h-[85vh] overflow-hidden">

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
      <div class="flex-1 overflow-y-auto min-h-0 pt-5 px-6 pb-2 flex flex-col gap-4">

        <!-- 샘플 다운로드 (항상 표시) -->
        <div class="flex items-center justify-between gap-3">
          <div class="text-base text-ink-4 leading-[1.6] [&_p]:m-0">
            <p>학생 명단이 담긴 CSV 또는 엑셀 파일을 업로드해 주세요.</p>
            <p>파일 내에 '학년, 반, 번호, 이름' 열이 포함되어 있는지 확인해 주세요.</p>
          </div>
          <button
              class="flex items-center gap-1.5 py-[7px] px-3 rounded-lg border border-blue/30 bg-blue/8 text-ink-3 text-sm cursor-pointer whitespace-nowrap transition-colors duration-150 shrink-0 hover:bg-blue/15"
              @click="downloadSample"
          >
            <Download :size="14"/>
            샘플 파일 다운로드
          </button>
        </div>

        <!-- 파일 미선택: 드롭존 -->
        <div
            v-if="!rawHeaders.length"
            class="border-2 border-dashed border-line rounded-[14px] py-12 px-6 flex flex-col items-center gap-2 cursor-pointer transition-colors duration-150 hover:border-blue/50 hover:bg-blue/4"
            :class="dragging ? 'border-blue/50 bg-blue/4' : ''"
            @dragover="onDragOver"
            @dragleave="onDragLeave"
            @drop="onDrop"
            @click="fileInputRef.click()"
        >
          <FileSpreadsheet :size="32" class="text-ink-5"/>
          <p class="text-base text-ink-3 m-0">
            파일을 여기에 드래그하거나 <span class="text-ink-2 underline">파일 선택</span>
          </p>
          <p class="text-sm text-ink-5 m-0">CSV, XLSX, XLS 지원</p>
        </div>

        <!-- 파일 선택됨: 2단 레이아웃 -->
        <div v-else class="flex gap-4 items-stretch">

          <!-- 좌: 열 매핑 -->
          <div class="w-[300px] shrink-0">
            <div class="h-full bg-blue/5 border border-blue/20 rounded-xl py-4 px-[18px] flex flex-col gap-3">
              <div class="flex items-center justify-between">
                <p class="text-sm font-semibold text-ink-3 m-0 tracking-[0.04em] uppercase">열 매핑 확인</p>
                <button
                    class="flex items-center gap-1 py-[5px] px-2.5 rounded-[7px] border border-blue/30 bg-blue/8 text-ink-3 text-sm cursor-pointer whitespace-nowrap transition-colors duration-150 hover:bg-blue/15"
                    @click="resetFile();"
                >
                  <FileSpreadsheet :size="13"/>
                  파일 초기화
                </button>
              </div>
              <p class="text-sm text-ink-4 -mt-1.5 truncate m-0">📄 {{ fileName }}</p>
              <div class="flex flex-col gap-2">
                <div
                    v-for="(label, field) in FIELD_LABELS"
                    :key="field"
                    class="flex items-center gap-2"
                >
                  <span class="text-base font-semibold text-ink-2 w-9 shrink-0">{{ label }}</span>
                  <select
                      class="flex-1 min-w-0 py-1.5 px-2 rounded-lg border bg-base text-ink text-sm outline-none cursor-pointer transition-colors duration-150 focus:border-blue/50"
                      :class="colMap[field] === null ? 'border-amber/30' : 'border-green/30'"
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
                  <span
                      v-if="colMap[field] !== null"
                      class="text-[11px] font-semibold rounded py-0.5 px-1 shrink-0 whitespace-nowrap text-green bg-green/10"
                  >자동</span>
                  <span
                      v-else
                      class="text-[11px] font-semibold rounded py-0.5 px-1 shrink-0 whitespace-nowrap text-amber bg-amber/10"
                  >미선택</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 우: 실시간 미리보기 -->
          <div class="flex-1 min-w-0">
            <div class="h-full bg-blue/5 border border-blue/20 rounded-xl py-4 px-[18px] flex flex-col gap-3">
              <p class="text-sm font-semibold text-ink-3 m-0 tracking-[0.04em] uppercase">미리보기</p>
              <p class="text-sm text-ink-4 -mt-1.5 m-0">
                <template v-if="allMapped">
                  총 {{ parsedRows.length }}행
                  <span v-if="errorRows.length > 0" class="text-red">(오류 {{ errorRows.length }}행 제외)</span>
                </template>
                <template v-else>열 매핑을 완료하면 전체 인원이 표시됩니다.</template>
              </p>
              <div class="border border-line rounded-[10px] overflow-hidden">
                <table class="w-full border-collapse [&_tr:last-child_td]:border-b-0">
                  <thead>
                  <tr>
                    <th class="text-xs font-semibold text-ink-4 py-2 px-2.5 bg-base border-b border-line text-left tracking-[0.04em] uppercase">행</th>
                    <th class="text-xs font-semibold text-ink-4 py-2 px-2.5 bg-base border-b border-line text-left tracking-[0.04em] uppercase">학년</th>
                    <th class="text-xs font-semibold text-ink-4 py-2 px-2.5 bg-base border-b border-line text-left tracking-[0.04em] uppercase">반</th>
                    <th class="text-xs font-semibold text-ink-4 py-2 px-2.5 bg-base border-b border-line text-left tracking-[0.04em] uppercase">번호</th>
                    <th class="text-xs font-semibold text-ink-4 py-2 px-2.5 bg-base border-b border-line text-left tracking-[0.04em] uppercase">이름</th>
                  </tr>
                  </thead>
                  <tbody>
                  <tr v-for="row in livePreviewRows" :key="row._row">
                    <td class="text-sm text-ink-4 py-[7px] px-2.5 border-b border-line/60">{{ row._row }}</td>
                    <td class="text-sm py-[7px] px-2.5 border-b border-line/60" :class="row.grade === null ? 'text-ink-4' : 'text-ink-2'">{{ row.grade ?? '—' }}</td>
                    <td class="text-sm py-[7px] px-2.5 border-b border-line/60" :class="row.classNum === null ? 'text-ink-4' : 'text-ink-2'">{{ row.classNum ?? '—' }}</td>
                    <td class="text-sm py-[7px] px-2.5 border-b border-line/60" :class="row.number === null ? 'text-ink-4' : 'text-ink-2'">{{ row.number ?? '—' }}</td>
                    <td class="text-sm py-[7px] px-2.5 border-b border-line/60" :class="row.name === null ? 'text-ink-4' : 'text-ink-2'">{{ row.name ?? '—' }}</td>
                  </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>

        <!-- 오류 -->
        <div v-if="parseError" class="msg-error flex items-center gap-2">
          <AlertCircle :size="15"/>
          {{ parseError }}
        </div>

        <!-- 가져오기 결과 -->
        <div v-if="importResult" class="msg-success flex items-center gap-2">
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
      <div class="flex items-center justify-end gap-2 px-6 pt-4 pb-5 border-t border-line shrink-0">
        <button class="btn-secondary" @click="emit('close')">닫기</button>
        <button
            class="btn-primary"
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
