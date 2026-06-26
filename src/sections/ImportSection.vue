<script setup>
import {computed, ref} from 'vue'
import {save} from '@tauri-apps/plugin-dialog'
import {useActivityStore} from '../stores/activity.js'
import {useRecordStore} from '../stores/record.js'
import {useFileStore} from '../stores/file.js'
import {Download, FileSpreadsheet} from 'lucide-vue-next'
import WizardLayout from '../components/WizardLayout.vue'
import DiffView from '../components/DiffView.vue'
import {Workbook} from 'exceljs'
import * as XLSX from 'xlsx'
import {SAMPLE_A_ROWS, SAMPLE_B_COLS, SAMPLE_B_ROWS} from '../data/sampleImportData'

// ── 스토어 ────────────────────────────────────────────────────

const activityStore = useActivityStore()
const recordStore = useRecordStore()
const fileStore = useFileStore()

const COL_ALIASES = {
  grade: ['학년', 'grade'],
  classNum: ['반', 'class', '학급', '반번호', 'classnum', 'class_num'],
  number: ['번호', 'number', 'num', '번', '출석번호'],
  name: ['이름', 'name', '성명', '학생명', '학생이름'],
  activityName: ['활동명', '활동 명', '활동', '분류', 'activity', 'activity_name', 'activityname'],
  activityContent: ['활동내용', '활동 내용', '내용', 'content', '기록', '활동기록'],
  studentId: ['학번', 'studentid', 'student_id'],
}

const FIELD_LABELS_A = {
  grade: '학년', classNum: '반', number: '번호',
  name: '이름 (선택)', activityName: '활동명', activityContent: '활동내용',
}
const FIELD_LABELS_B = {
  grade: '학년', classNum: '반', number: '번호', name: '이름 (선택)',
}
const REQUIRED_A = ['grade', 'classNum', 'number', 'activityName', 'activityContent']
const REQUIRED_B = ['grade', 'classNum', 'number']

// ── 상태 ──────────────────────────────────────────────────────

const step = ref(1)
const previewCollapsed = ref(false)
const idMode = ref('fields') // 'fields' | 'studentId'
const dragging = ref(false)
const fileName = ref('')
const parseError = ref('')
const fileInputRef = ref(null)

const rawHeaders = ref([])
const rawData = ref([])

const fileType = ref(null)

const colMap = ref({
  grade: null, classNum: null, number: null, name: null,
  activityName: null, activityContent: null, studentId: null,
})

const activityMatchMap = ref({})
const dbActivities = ref([])

const importing = ref(false)
const importResult = ref(null)
const importError = ref('')
const isNavigating = ref(false)

// ── Diff 미리보기 상태 ────────────────────────────────────────
const previewLoading = ref(false)
const previewError = ref('')
const previewItems = ref([])
const checkedKeys = ref(new Set())
const diffViewMode = ref('raw')
const pendingRecords = ref([])

// ── Computed ──────────────────────────────────────────────────

const previewRows = computed(() => rawData.value.slice(0, 5))

const hasDuplicateCols = computed(() => {
  const targetKeys = {
    fields: ['grade', 'classNum', 'number', 'name', 'activityName', 'activityContent'],
    studentId: ['studentId', 'name', 'activityName', 'activityContent']
  }
  const currentKeys = targetKeys[idMode.value] || []
  const vals = currentKeys
      .map(key => colMap.value[key])
      .filter(v => v !== null && v !== undefined && v !== '')
  return vals.length !== new Set(vals).size
})

const canGoNext = computed(() => {
  if (step.value === 1) return rawHeaders.value.length > 0
  if (step.value === 2) return fileType.value !== null
  if (step.value === 3) {
    if (hasDuplicateCols.value) return false
    const m = colMap.value
    if (idMode.value === 'studentId') {
      if (m.studentId === null) return false
      if (fileType.value === 'A') return m.activityName !== null && m.activityContent !== null
      return true
    }
    const required = fileType.value === 'A' ? REQUIRED_A : REQUIRED_B
    return required.every(f => m[f] !== null)
  }
  if (step.value === 4) return extractedActivities.value.length > 0
  if (step.value === 5) return !previewLoading.value
  return false
})

const extractedActivities = computed(() => {
  if (fileType.value === 'A') {
    const col = colMap.value.activityName
    if (col === null) return []
    const set = new Set()
    rawData.value.forEach(row => {
      const v = String(row[col] ?? '').trim()
      if (v) set.add(v)
    })
    return [...set]
  } else {
    const m = colMap.value
    const identityCols = idMode.value === 'studentId'
        ? [m.studentId, m.name]
        : [m.grade, m.classNum, m.number, m.name]
    const mapped = new Set(identityCols.filter(v => v !== null))
    return rawHeaders.value.filter((_, i) => !mapped.has(i))
  }
})

const activityColIndices = computed(() => {
  if (fileType.value !== 'B') return []
  const m = colMap.value
  const identityCols = idMode.value === 'studentId'
      ? [m.studentId, m.name]
      : [m.grade, m.classNum, m.number, m.name]
  const mapped = new Set(identityCols.filter(v => v !== null))
  return rawHeaders.value
      .map((name, index) => ({name, index}))
      .filter(({index}) => !mapped.has(index))
})

const changedPreviewItems = computed(() =>
    previewItems.value.filter(item => item.existing_content !== '' && item.existing_content !== item.new_content)
)
const newPreviewItemsCount = computed(() =>
    previewItems.value.filter(item => item.existing_content === '').length
)
const unchangedCount = computed(() =>
    previewItems.value.filter(item => item.existing_content !== '' && item.existing_content === item.new_content).length
)
const checkedChangedCount = computed(() =>
    changedPreviewItems.value.filter(item => checkedKeys.value.has(item.key)).length
)
const allChangedChecked = computed(() =>
    changedPreviewItems.value.length > 0 &&
    changedPreviewItems.value.every(item => checkedKeys.value.has(item.key))
)

const studentIdPreviewRows = computed(() => {
  const col = colMap.value.studentId
  if (col === null) return []
  return rawData.value.slice(0, 5).map(row => {
    const raw = row[col]
    const parsed = parseStudentId(raw)
    return parsed ? {raw, ...parsed, error: false} : {raw, grade: '?', classNum: '?', number: '?', error: true}
  })
})

// ── 공통 헬퍼 ────────────────────────────────────────────────

function resolveIdentity(row) {
  const m = colMap.value
  if (idMode.value === 'studentId') {
    const p = parseStudentId(row[m.studentId])
    if (!p) return null
    return {grade: p.grade, class_num: p.classNum, number: p.number}
  }
  const grade = Number(row[m.grade])
  const class_num = Number(row[m.classNum])
  const number = Number(row[m.number])
  if (!grade || !class_num || !number) return null
  return {grade, class_num, number}
}

// ── 파일 처리 ─────────────────────────────────────────────────

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
        if (ch === '"' && line[i + 1] === '"') {
          field += '"';
          i++
        } else if (ch === '"') inQuotes = false
        else field += ch
      } else {
        if (ch === '"') inQuotes = true
        else if (ch === ',') {
          row.push(field);
          field = ''
        } else field += ch
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

async function loadXlsxRows(buffer) {
  try {
    const workbook = new Workbook()
    await workbook.xlsx.load(buffer)
    const worksheet = workbook.worksheets[0]
    const rows = []
    worksheet.eachRow((row) => {
      rows.push(row.values.slice(1).map(cellValue))
    })
    if (rows.length > 1) {
      const headerLen = rows[0].length
      for (let i = 1; i < rows.length; i++) {
        while (rows[i].length < headerLen) rows[i].push('')
      }
    }
    return rows
  } catch {
    // 한셀 등 비표준 xlsx 폴백
    const wb = XLSX.read(buffer, {type: 'array'})
    const ws = wb.Sheets[wb.SheetNames[0]]
    const raw = XLSX.utils.sheet_to_json(ws, {header: 1, defval: ''})
    const rows = raw.map(row => row.map(v => (v === null || v === undefined) ? '' : String(v)))
    if (rows.length > 1) {
      const headerLen = rows[0].length
      for (let i = 1; i < rows.length; i++) {
        while (rows[i].length < headerLen) rows[i].push('')
      }
    }
    return rows
  }
}

function bufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  const chunk = 8192
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(binary)
}

function processFile(file) {
  fileName.value = file.name
  parseError.value = ''
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
  reader.onerror = () => {
    parseError.value = '파일을 읽을 수 없습니다. 파일 권한을 확인해주세요.'
  }
  reader.onload = async (ev) => {
    try {
      let rows
      if (ext === 'csv') {
        rows = parseCsv(decodeCSVBytes(ev.target.result))
      } else {
        rows = await loadXlsxRows(ev.target.result)
      }
      if (rows.length < 2) {
        parseError.value = '데이터가 없습니다. 헤더 포함 최소 2행이 필요합니다.'
        return
      }
      rawHeaders.value = rows[0].map(h => String(h ?? '').trim())
      rawData.value = rows.slice(1)
      autoDetectColumns()
    } catch (err) {
      parseError.value = '파싱 오류: ' + err.message
    }
  }
  reader.readAsArrayBuffer(file)
}

function parseStudentId(val) {
  const s = String(val ?? '').trim().replace(/\D/g, '')
  if (s.length === 4) return {grade: +s[0], classNum: +s[1], number: +s.slice(2, 4)}
  if (s.length === 5) return {grade: +s[0], classNum: +s.slice(1, 3), number: +s.slice(3, 5)}
  if (s.length === 6) return {grade: +s[0], classNum: +s.slice(1, 3), number: +s.slice(3, 6)}
  return null
}

function autoDetectColumns() {
  const map = {
    grade: null, classNum: null, number: null, name: null,
    activityName: null, activityContent: null, studentId: null,
  }
  rawHeaders.value.forEach((header, idx) => {
    const h = header.toLowerCase().replace(/\s/g, '')
    for (const [field, aliases] of Object.entries(COL_ALIASES)) {
      if (map[field] === null && aliases.includes(h)) {
        map[field] = idx
      }
    }
  })
  colMap.value = map
  if (map.studentId !== null && map.grade === null && map.classNum === null && map.number === null) {
    idMode.value = 'studentId'
  } else {
    idMode.value = 'fields'
  }
}

// ── 네비게이션 ────────────────────────────────────────────────

async function goNext() {
  if (!canGoNext.value) return
  if (isNavigating.value) return
  isNavigating.value = true

  parseError.value = ''
  previewError.value = ''
  importError.value = ''

  try {
    if (step.value === 3) {
      await loadDbActivities()
      if (importError.value) return
      initActivityMatchMap()
    }
    if (step.value === 4) {
      await loadPreview()
    }
    step.value++
  } finally {
    isNavigating.value = false
  }
}

function goPrev() {
  if (step.value > 1) step.value--
}

async function loadDbActivities() {
  try {
    await activityStore.fetchActivities()
    dbActivities.value = activityStore.activities.map(a => ({id: a.id, name: a.name}))
  } catch (e) {
    importError.value = '활동 목록 로드 실패: ' + String(e)
  }
}

function initActivityMatchMap() {
  const map = {}
  for (const actName of extractedActivities.value) {
    const match = dbActivities.value.find(a => a.name === actName)
    map[actName] = match ? match.id : 0
  }
  activityMatchMap.value = map
}

// ── Diff 미리보기 로드 ────────────────────────────────────────

function toggleAllChanged() {
  const s = new Set(checkedKeys.value)
  if (allChangedChecked.value) {
    changedPreviewItems.value.forEach(item => s.delete(item.key))
  } else {
    changedPreviewItems.value.forEach(item => s.add(item.key))
  }
  checkedKeys.value = s
}

function toggleItem(key) {
  const s = new Set(checkedKeys.value)
  if (s.has(key)) s.delete(key)
  else s.add(key)
  checkedKeys.value = s
}

async function loadPreview() {
  previewLoading.value = true
  previewError.value = ''
  previewItems.value = []
  checkedKeys.value = new Set()
  pendingRecords.value = []

  try {
    const m = colMap.value
    const existingActRecords = []
    const newActRawItems = []

    function buildPending(row, identity, actName, activity_id, content) {
      const name = m.name !== null ? (String(row[m.name] ?? '').trim() || null) : null
      const previewKey = activity_id > 0
          ? `${activity_id}-${identity.grade}-${identity.class_num}-${identity.number}`
          : `new-${actName}-${identity.grade}-${identity.class_num}-${identity.number}`
      return {
        grade: identity.grade,
        class_num: identity.class_num,
        number: identity.number,
        name,
        activity_id,
        activity_name: actName,
        content,
        previewKey
      }
    }

    if (fileType.value === 'A') {
      for (const row of rawData.value) {
        const identity = resolveIdentity(row)
        if (!identity) continue
        const actName = String(row[m.activityName] ?? '').trim()
        const content = String(row[m.activityContent] ?? '').trim()
        if (!actName || !content) continue
        const activity_id = activityMatchMap.value[actName]
        if (activity_id === undefined) continue
        const rec = buildPending(row, identity, actName, activity_id, content)
        if (activity_id > 0) existingActRecords.push(rec)
        else newActRawItems.push(rec)
      }
    } else {
      for (const row of rawData.value) {
        const identity = resolveIdentity(row)
        if (!identity) continue
        for (const {name: actName, index} of activityColIndices.value) {
          const content = String(row[index] ?? '').trim()
          if (!content) continue
          const activity_id = activityMatchMap.value[actName]
          if (activity_id === undefined) continue
          const rec = buildPending(row, identity, actName, activity_id, content)
          if (activity_id > 0) existingActRecords.push(rec)
          else newActRawItems.push(rec)
        }
      }
    }

    pendingRecords.value = [...existingActRecords, ...newActRawItems]

    let backendItems = []
    if (existingActRecords.length > 0) {
      backendItems = await recordStore.previewImportRecords(
          existingActRecords.map(r => ({
            grade: r.grade, class_num: r.class_num, number: r.number,
            name: r.name, activity_id: r.activity_id, content: r.content,
          }))
      )
    }

    const allItems = [
      ...backendItems.map(item => ({
        key: `${item.activity_id}-${item.grade}-${item.class_num}-${item.number}`,
        grade: item.grade,
        class_num: item.class_num,
        number: item.number,
        student_name: item.student_name,
        activity_id: item.activity_id,
        activity_name: item.activity_name,
        new_content: item.new_content,
        existing_content: item.existing_content,
      })),
      ...newActRawItems.map(r => ({
        key: r.previewKey,
        grade: r.grade,
        class_num: r.class_num,
        number: r.number,
        student_name: r.name || `${r.grade}학년 ${r.class_num}반 ${r.number}번`,
        activity_id: r.activity_id,
        activity_name: r.activity_name,
        new_content: r.content,
        existing_content: '',
      })),
    ]

    previewItems.value = allItems

    const initChecked = new Set()
    for (const item of allItems) {
      if (item.existing_content !== '' && item.existing_content !== item.new_content) {
        initChecked.add(item.key)
      }
    }
    checkedKeys.value = initChecked

  } catch (e) {
    previewError.value = '미리보기 로드 실패: ' + String(e)
  } finally {
    previewLoading.value = false
  }
}

// ── 가져오기 실행 ─────────────────────────────────────────────

async function doImport() {
  importing.value = true
  importError.value = ''
  importResult.value = null

  try {
    const finalMap = {...activityMatchMap.value}
    for (const [name, id] of Object.entries(finalMap)) {
      if (id === 0) {
        finalMap[name] = await activityStore.createActivity(name)
      }
    }

    const records = []
    for (const pending of pendingRecords.value) {
      const activity_id = pending.activity_id > 0
          ? pending.activity_id
          : finalMap[pending.activity_name]
      if (!activity_id) continue

      const previewItem = previewItems.value.find(p => p.key === pending.previewKey)

      let include = false
      if (!previewItem || previewItem.existing_content === '') {
        include = true
      } else if (previewItem.existing_content === previewItem.new_content) {
        include = false
      } else {
        include = checkedKeys.value.has(pending.previewKey)
      }

      if (include) {
        records.push({
          grade: pending.grade,
          class_num: pending.class_num,
          number: pending.number,
          name: pending.name,
          activity_id,
          content: pending.content,
        })
      }
    }

    if (records.length === 0) {
      importError.value = '가져올 데이터가 없습니다.'
      return
    }

    importResult.value = await recordStore.bulkImportRecords(records)
    step.value++
  } catch (e) {
    importError.value = '가져오기 실패: ' + String(e)
  } finally {
    importing.value = false
  }
}

// ── 예시 파일 다운로드 ─────────────────────────────────────────

async function downloadSampleA() {
  const filePath = await save({
    title: 'A타입 예시 파일 저장',
    defaultPath: '예시_A타입.xlsx',
    filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
  })

  if (!filePath) return

  try {
    const headers = ['학년', '반', '번호', '이름', '활동명', '활동내용']
    const workbook = new Workbook()
    const worksheet = workbook.addWorksheet('예시')
    worksheet.addRow(headers)
    for (const row of SAMPLE_A_ROWS) {
      worksheet.addRow(headers.map(h => row[h]))
    }
    const buffer = await workbook.xlsx.writeBuffer()
    const data = bufferToBase64(buffer)
    await fileStore.writeBytesFile(filePath, data)
  } catch (e) {
    parseError.value = `파일 저장 실패: ${e}`
  }
}

async function downloadSampleB() {
  const filePath = await save({
    title: 'B타입 예시 파일 저장',
    defaultPath: '예시_B타입.xlsx',
    filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
  })
  if (!filePath) return
  try {
    const workbook = new Workbook()
    const worksheet = workbook.addWorksheet('예시')
    worksheet.addRow(SAMPLE_B_COLS)
    for (const row of SAMPLE_B_ROWS) {
      worksheet.addRow(SAMPLE_B_COLS.map(c => row[c]))
    }
    const buffer = await workbook.xlsx.writeBuffer()
    const data = bufferToBase64(buffer)
    await fileStore.writeBytesFile(filePath, data)
  } catch (e) {
    parseError.value = `파일 저장 실패: ${e}`
  }
}

function resetWizard() {
  step.value = 1
  fileName.value = ''
  parseError.value = ''
  rawHeaders.value = []
  rawData.value = []
  fileType.value = null
  idMode.value = 'fields'
  colMap.value = {
    grade: null,
    classNum: null,
    number: null,
    name: null,
    activityName: null,
    activityContent: null,
    studentId: null
  }
  activityMatchMap.value = {}
  dbActivities.value = []
  importing.value = false
  importResult.value = null
  importError.value = ''
  isNavigating.value = false
  previewLoading.value = false
  previewError.value = ''
  previewItems.value = []
  checkedKeys.value = new Set()
  diffViewMode.value = 'raw'
  pendingRecords.value = []
}
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden box-border">

    <!-- 툴바 -->
    <div class="flex items-center justify-between px-10 pt-9 pb-6 border-b border-line shrink-0">
      <div class="flex flex-col">
        <h2 class="text-[22px] font-bold text-ink m-0 mb-1.5">데이터 가져오기(Import)</h2>
        <p class="text-base text-ink-3 m-0">다양한 형식의 학교생활기록부 기재 문장을 본 프로그램으로 가져옵니다.</p>
      </div>
    </div>

    <WizardLayout
        :stepCount="6"
        :currentStep="step"
        :canGoNext="canGoNext"
        :isNavigating="isNavigating"
        :showFooter="!importResult"
        @prev="goPrev"
        @next="goNext"
    >

      <!-- Step 1: 파일 업로드 -->
      <div v-if="step === 1">

        <!-- 예시 파일 다운로드 -->
        <details class="sample-section mb-1.5">
          <summary class="sample-section-summary flex items-center cursor-pointer list-none select-none mb-1.5 [&::-webkit-details-marker]:hidden">
            <h3 class="text-lg font-bold text-ink m-0">Step 0. 가져오기(Import) 가능한 파일 안내</h3>
            <span class="sample-section-toggle-label ml-4 text-base text-ink-5 whitespace-nowrap font-normal"></span>
          </summary>
          <p class="text-base text-ink-5 m-0 mb-6">가져오기 가능한 파일은 두 가지 유형(행 단위, 열 단위)입니다. 예시 다운로드 버튼을 눌러 각 유형의 예시를 확인하세요.</p>

          <div class="grid gap-4 grid-duo-card">
            <!-- A 타입 (예시용, 클릭 불가) -->
            <div class="border-2 border-line rounded-xl p-5 cursor-default">
              <div class="flex items-center gap-2.5 mb-2.5">
                <span class="text-xs font-bold rounded-[6px] py-[2px] px-2 text-red bg-red/[0.12] border border-red/35">A 타입</span>
                <span class="text-base font-semibold text-ink">행 단위 활동 형식</span>
              </div>
              <p class="text-sm text-ink-5 m-0 mb-3.5 leading-relaxed">한 행에 학생 1명의 활동 1개를 기재합니다.<br>학생 1명의 활동은 여러 행에 걸쳐 기록됩니다(학생 1명 = 여러 행).</p>
              <div class="overflow-x-auto border border-line rounded-[6px] [&_tr:last-child_td]:border-b-0">
                <table class="border-collapse w-full text-xs">
                  <thead><tr>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학년</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">반</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">번호</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">이름</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">활동명</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">활동내용</th>
                  </tr></thead>
                  <tbody>
                  <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">현장체험학습</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">지역 기관을 탐방...</td></tr>
                  <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학급자치회</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">회의에 적극 참여...</td></tr>
                  <tr><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">2</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학생B</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">체육대회</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">다양한 종목에 참여...</td></tr>
                  </tbody>
                </table>
              </div>
              <button class="flex items-center gap-1.5 mt-3.5 py-[7px] px-3.5 bg-blue/[0.08] border border-blue/25 rounded-[7px] text-blue-2 text-sm cursor-pointer transition-[background-color,color] hover:bg-blue/[0.18]" @click.stop="downloadSampleA">
                <Download :size="13"/>
                A타입 예시 다운로드
              </button>
            </div>

            <!-- B 타입 (예시용, 클릭 불가) -->
            <div class="border-2 border-line rounded-xl p-5 cursor-default">
              <div class="flex items-center gap-2.5 mb-2.5">
                <span class="text-xs font-bold rounded-[6px] py-[2px] px-2 text-amber bg-amber/[0.15] border border-amber/40">B 타입</span>
                <span class="text-base font-semibold text-ink">열 단위 활동 형식</span>
              </div>
              <p class="text-sm text-ink-5 m-0 mb-3.5 leading-relaxed">활동이 열(헤더)로 구분된 형식입니다.<br>학생 1명의 모든 활동이 한 행에 기록됩니다(학생 1명 = 1행).</p>
              <div class="overflow-x-auto border border-line rounded-[6px] [&_tr:last-child_td]:border-b-0">
                <table class="border-collapse w-full text-xs">
                  <thead><tr>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학년</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">반</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">번호</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">이름</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">현장체험학습</th>
                    <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학급자치회</th>
                  </tr></thead>
                  <tbody>
                  <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">지역 기관을...</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">회의에 적극...</td></tr>
                  <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">2</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생B</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap"></td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap"></td></tr>
                  <tr><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">5</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학생E</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap"></td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학급 행사 준비...</td></tr>
                  </tbody>
                </table>
              </div>
              <button class="flex items-center gap-1.5 mt-3.5 py-[7px] px-3.5 bg-blue/[0.08] border border-blue/25 rounded-[7px] text-blue-2 text-sm cursor-pointer transition-[background-color,color] hover:bg-blue/[0.18]" @click.stop="downloadSampleB">
                <Download :size="13"/>
                B타입 예시 다운로드
              </button>
            </div>
          </div>
        </details>

        <div class="mb-3 pb-3 border-b border-line"></div>

        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 1. 가져올 파일 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">CSV 또는 XLSX 파일을 선택하거나 드래그하세요.</p>

        <input ref="fileInputRef" type="file" accept=".csv,.xlsx,.xls" class="hidden" @change="onFileChange"/>

        <div
            class="border-2 rounded-xl py-12 px-6 text-center cursor-pointer transition-[border-color,background-color] duration-200 flex flex-col items-center gap-2"
            :class="rawHeaders.length > 0 ? ['border-green/40', 'border-solid'] : dragging ? ['border-dashed','border-blue/50','bg-blue/[0.04]'] : ['border-dashed','border-line','hover:border-blue/50','hover:bg-blue/[0.04]']"
            @dragover="onDragOver"
            @dragleave="onDragLeave"
            @drop="onDrop"
            @click="fileInputRef.click()"
        >
          <FileSpreadsheet :size="40" class="text-ink-5"/>
          <p v-if="!fileName" class="text-base text-ink-5 m-0">파일을 여기에 드래그하거나 클릭하여 선택</p>
          <p v-else class="text-base font-semibold text-ink-2 m-0">{{ fileName }}</p>
          <p class="text-sm text-ink-5 m-0">CSV, XLSX 지원</p>
        </div>

        <p v-if="parseError" class="text-sm text-red my-3 mb-6">{{ parseError }}</p>

        <div v-if="rawHeaders.length > 0">
          <div class="mb-3 pb-3 border-b border-line"></div>
          <p class="text-sm text-ink-5 m-0 mb-2.5">미리보기 (첫 {{ Math.min(rawData.length, 5) }}행)</p>
          <div class="overflow-x-auto border border-line rounded-lg">
            <table class="border-collapse min-w-full text-sm">
              <thead><tr>
                <th v-for="h in rawHeaders" :key="h" class="py-2 px-3 bg-surface text-ink-5 font-semibold text-left whitespace-nowrap border-b border-line">{{ h }}</th>
              </tr></thead>
              <tbody>
              <tr v-for="(row, i) in previewRows" :key="i">
                <td v-for="(cell, j) in row" :key="j" class="py-[7px] px-3 text-ink-2 border-b border-line/70 whitespace-nowrap max-w-[200px] overflow-hidden text-ellipsis">{{ cell }}</td>
              </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- Step 2: 양식 선택 -->
      <div v-else-if="step === 2">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 2. 엑셀 파일 양식 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">가져올 엑셀 파일 양식에 맞는 타입을 선택하세요. 잘못된 타입을 선택하면 데이터 오류 또는 항목 누락이 발생할 수 있습니다.</p>

        <div class="grid gap-4 grid-duo-card">
          <!-- A 타입 -->
          <div
              class="border-2 rounded-xl p-5 cursor-pointer transition-[border-color,background-color] duration-200"
              :class="fileType === 'A' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="fileType = 'A'"
          >
            <div class="flex items-center gap-2.5 mb-2.5">
              <span class="text-xs font-bold rounded-[6px] py-[2px] px-2 text-red bg-red/[0.12] border border-red/35">A 타입</span>
              <span class="text-base font-semibold text-ink">행 단위 활동 형식</span>
            </div>
            <p class="text-sm text-ink-5 m-0 mb-3.5 leading-relaxed">한 행에 학생 1명의 활동 1개를 기재합니다.<br>학생 1명의 활동은 여러 행에 걸쳐 기록됩니다(학생 1명 = 여러 행).</p>
            <div class="overflow-x-auto border border-line rounded-[6px] [&_tr:last-child_td]:border-b-0">
              <table class="border-collapse w-full text-xs">
                <thead><tr>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학년</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">반</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">번호</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">이름</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">활동명</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">활동내용</th>
                </tr></thead>
                <tbody>
                <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">현장체험학습</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">지역 기관을 탐방...</td></tr>
                <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학급자치회</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">회의에 적극 참여...</td></tr>
                <tr><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">2</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학생B</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">체육대회</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">다양한 종목에 참여...</td></tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- B 타입 -->
          <div
              class="border-2 rounded-xl p-5 cursor-pointer transition-[border-color,background-color] duration-200"
              :class="fileType === 'B' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="fileType = 'B'"
          >
            <div class="flex items-center gap-2.5 mb-2.5">
              <span class="text-xs font-bold rounded-[6px] py-[2px] px-2 text-amber bg-amber/[0.15] border border-amber/40">B 타입</span>
              <span class="text-base font-semibold text-ink">열 단위 활동 형식</span>
            </div>
            <p class="text-sm text-ink-5 m-0 mb-3.5 leading-relaxed">활동이 열(헤더)로 구분된 형식입니다.<br>학생 1명의 모든 활동이 한 행에 기록됩니다(학생 1명 = 1행).</p>
            <div class="overflow-x-auto border border-line rounded-[6px] [&_tr:last-child_td]:border-b-0">
              <table class="border-collapse w-full text-xs">
                <thead><tr>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학년</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">반</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">번호</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">이름</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">현장체험학습</th>
                  <th class="py-1.5 px-2 bg-base text-ink-5 font-semibold text-left border-b border-line whitespace-nowrap">학급자치회</th>
                </tr></thead>
                <tbody>
                <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생A</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">지역 기관을...</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">회의에 적극...</td></tr>
                <tr><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">2</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap">학생B</td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap"></td><td class="py-[5px] px-2 text-ink-3 border-b border-line/50 whitespace-nowrap"></td></tr>
                <tr><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">3</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">1</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">5</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학생E</td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap"></td><td class="py-[5px] px-2 text-ink-3 whitespace-nowrap">학급 행사 준비...</td></tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <!-- Step 3: 열 매핑 -->
      <div v-else-if="step === 3">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 3. 열 매핑</h3>
        <p class="text-base text-ink-5 m-0 mb-6">엑셀 파일의 각 열을 올바르게 연결하세요. <span class="text-red ml-0.5">*</span> 는 필수입니다.</p>

        <!-- 학생 식별 방식 토글 -->
        <div class="flex items-center gap-4 mb-6">
          <span class="text-base text-ink-2 whitespace-nowrap">학생 식별 방식</span>
          <div class="flex gap-0 border border-line rounded-lg overflow-hidden">
            <button
                class="py-[7px] px-4 bg-transparent border-none text-sm cursor-pointer transition-[background-color,color]"
                :class="idMode === 'fields' ? 'bg-blue/[0.12] text-blue-2' : 'text-ink-5'"
                @click="idMode = 'fields'"
            >학년 · 반 · 번호
            </button>
            <button
                class="py-[7px] px-4 bg-transparent border-none text-sm cursor-pointer transition-[background-color,color] border-l border-line"
                :class="idMode === 'studentId' ? 'bg-blue/[0.12] text-blue-2' : 'text-ink-5'"
                @click="idMode = 'studentId'"
            >학번
            </button>
          </div>
        </div>

        <div class="flex flex-col gap-3">
          <!-- 학번 모드: 단일 열 선택 + 파싱 미리보기 -->
          <template v-if="idMode === 'studentId'">
            <div class="grid grid-cols-[160px_1fr] items-center gap-4">
              <label class="text-base text-ink-2">학번 <span class="text-red ml-0.5">*</span></label>
              <select class="py-2 px-3 bg-surface border border-line rounded-lg text-ink text-sm outline-none cursor-pointer focus:border-blue/50" v-model="colMap['studentId']">
                <option :value="null">— 선택 안 함 —</option>
                <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
              </select>
            </div>
            <div v-if="colMap['studentId'] !== null" class="mt-1 py-3.5 px-4 bg-surface border border-line rounded-lg">
              <p class="text-sm text-ink-5 m-0 mb-2.5">파싱 미리보기 (ABCC·4자리, ABBCC·5자리, ABBCCC·6자리 / A=학년, B=반, C=번호)</p>
              <table class="border-collapse w-auto text-sm sid-preview-table">
                <thead><tr>
                  <th class="py-2 px-3 bg-base text-ink-5 font-semibold text-left whitespace-nowrap border-b border-line">학번 원본</th>
                  <th class="py-2 px-3 bg-base text-ink-5 font-semibold text-left whitespace-nowrap border-b border-line">학년</th>
                  <th class="py-2 px-3 bg-base text-ink-5 font-semibold text-left whitespace-nowrap border-b border-line">반</th>
                  <th class="py-2 px-3 bg-base text-ink-5 font-semibold text-left whitespace-nowrap border-b border-line">번호</th>
                </tr></thead>
                <tbody>
                <tr v-for="(r, i) in studentIdPreviewRows" :key="i" :class="{ 'sid-row-error': r.error }">
                  <td class="py-[7px] px-3 text-ink-2 border-b border-line/70 whitespace-nowrap">{{ r.raw }}</td>
                  <td class="py-[7px] px-3 text-ink-2 border-b border-line/70 whitespace-nowrap">{{ r.grade }}</td>
                  <td class="py-[7px] px-3 text-ink-2 border-b border-line/70 whitespace-nowrap">{{ r.classNum }}</td>
                  <td class="py-[7px] px-3 text-ink-2 border-b border-line/70 whitespace-nowrap">{{ r.number }}</td>
                </tr>
                </tbody>
              </table>
            </div>
          </template>
          <!-- 필드 직접 지정 모드 -->
          <template v-else>
            <div v-for="field in ['grade','classNum','number']" :key="field" class="grid grid-cols-[160px_1fr] items-center gap-4">
              <label class="text-base text-ink-2">{{ FIELD_LABELS_A[field] }} <span class="text-red ml-0.5">*</span></label>
              <select class="py-2 px-3 bg-surface border border-line rounded-lg text-ink text-sm outline-none cursor-pointer focus:border-blue/50" v-model="colMap[field]">
                <option :value="null">— 선택 안 함 —</option>
                <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
              </select>
            </div>
          </template>

          <!-- 이름 (공통, 선택) -->
          <div class="grid grid-cols-[160px_1fr] items-center gap-4">
            <label class="text-base text-ink-2">이름 (선택)</label>
            <select class="py-2 px-3 bg-surface border border-line rounded-lg text-ink text-sm outline-none cursor-pointer focus:border-blue/50" v-model="colMap['name']">
              <option :value="null">— 선택 안 함 —</option>
              <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
            </select>
          </div>

          <!-- A타입 활동 필드 -->
          <template v-if="fileType === 'A'">
            <div v-for="field in ['activityName','activityContent']" :key="field" class="grid grid-cols-[160px_1fr] items-center gap-4">
              <label class="text-base text-ink-2">{{ FIELD_LABELS_A[field] }} <span class="text-red ml-0.5">*</span></label>
              <select class="py-2 px-3 bg-surface border border-line rounded-lg text-ink text-sm outline-none cursor-pointer focus:border-blue/50" v-model="colMap[field]">
                <option :value="null">— 선택 안 함 —</option>
                <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
              </select>
            </div>
          </template>
          <!-- B타입 활동 열 자동 표시 -->
          <template v-else>
            <div class="grid grid-cols-[160px_1fr] items-center gap-4">
              <label class="text-base text-ink-2">활동 열 (자동)</label>
              <div class="flex flex-wrap gap-1.5 items-center min-h-9">
                <span v-if="extractedActivities.length === 0" class="text-sm text-ink-5">학생 정보 열을 선택하면 나머지가 활동 열로 지정됩니다.</span>
                <span v-for="n in extractedActivities" :key="n"
                      class="text-sm text-blue-2 bg-blue/[0.10] border border-blue/25 rounded-[6px] py-[3px] px-2">{{ n }}</span>
              </div>
            </div>
          </template>

          <p v-if="hasDuplicateCols" class="text-sm text-red m-0 mt-2">동일한 열을 여러 필드에 지정할 수 없습니다. 각 필드에 서로 다른 열을 선택해 주세요.</p>
          <p v-if="importError" class="text-sm text-red m-0 mt-2">{{ importError }}</p>
        </div>
      </div>

      <!-- Step 4: 활동 매칭 -->
      <div v-else-if="step === 4">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 4. 활동 매칭</h3>
        <p class="text-base text-ink-5 m-0 mb-6">파일의 활동명을 기존 활동에 연결하거나 새로 만드세요. 이름이 일치하면 자동 매칭됩니다.</p>

        <p v-if="extractedActivities.length === 0" class="text-base text-ink-5 m-0">파일에서 활동을 찾을 수 없습니다. 이전 단계로 돌아가 열 매핑을 확인하세요.</p>

        <div v-else class="flex flex-col border border-line rounded-[10px] overflow-hidden">
          <div class="grid items-center gap-3 py-2.5 px-4 bg-surface text-sm font-semibold text-ink-5 border-b border-line grid-mapper">
            <span>파일의 활동명</span>
            <span></span>
            <span>연결할 활동</span>
          </div>
          <div v-for="actName in extractedActivities" :key="actName"
               class="grid items-center gap-3 py-2.5 px-4 border-b border-line/70 last:border-b-0 grid-mapper">
            <span class="text-base text-ink-2">{{ actName }}</span>
            <span class="text-base text-ink-5 text-center">→</span>
            <select class="py-[7px] px-2.5 bg-surface border border-line rounded-lg text-ink text-sm outline-none cursor-pointer w-full focus:border-blue/50"
                    v-model="activityMatchMap[actName]">
              <option :value="0">＋ 새로 만들기</option>
              <option v-for="a in dbActivities" :key="a.id" :value="a.id">{{ a.name }}</option>
            </select>
          </div>
        </div>
      </div>

      <!-- Step 5: 변경사항 확인 -->
      <div v-else-if="step === 5">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 5. 변경사항 확인</h3>
        <p class="text-base text-ink-5 m-0 mb-6">기존 데이터와 비교하여 변경될 항목을 확인하고 업데이트할 항목을 선택하세요.</p>

        <div v-if="previewLoading" class="py-12 text-center text-ink-5 text-base">기존 데이터와 비교 중...</div>
        <p v-if="previewError" class="text-sm text-red my-3 mb-6">
          {{ previewError }}<br>
          <span class="text-xs text-ink-5">미리보기 오류가 있어도 다음 단계로 진행하면 모든 항목이 가져와집니다.</span>
        </p>

        <template v-if="!previewLoading">
          <!-- 통계 바 -->
          <div class="flex gap-2.5 mb-5 flex-wrap">
            <span class="text-sm font-semibold py-1 px-2.5 rounded-[6px] border text-amber bg-amber/[0.10] border-amber/30">변경 {{ changedPreviewItems.length }}건</span>
            <span class="text-sm font-semibold py-1 px-2.5 rounded-[6px] border text-green bg-green/[0.10] border-green/30">신규 {{ newPreviewItemsCount }}건</span>
            <span class="text-sm font-semibold py-1 px-2.5 rounded-[6px] border text-ink-5 bg-line/20 border-line">동일 {{ unchangedCount }}건</span>
          </div>

          <!-- 변경 항목 없음 -->
          <div v-if="changedPreviewItems.length === 0" class="py-10 text-center">
            <p class="text-base text-ink m-0 mb-2">변경되는 항목이 없습니다.</p>
            <p v-if="newPreviewItemsCount > 0" class="text-base text-ink-5 m-0 mt-1">신규 기록 {{ newPreviewItemsCount }}건이 자동으로 추가됩니다.</p>
            <p v-if="unchangedCount > 0" class="text-base text-ink-5 m-0 mt-1">{{ unchangedCount }}건은 기존과 동일하여 건너뜁니다.</p>
          </div>

          <!-- 변경 항목 목록 -->
          <template v-else>
            <div class="flex items-center justify-between mb-3.5 flex-wrap gap-2.5">
              <label class="flex items-center gap-2 text-sm text-ink-2 cursor-pointer select-none">
                <input
                    type="checkbox"
                    class="w-[15px] h-[15px] cursor-pointer accent-blue"
                    :checked="allChangedChecked"
                    @change="toggleAllChanged"
                />
                전체 선택/해제
                <span class="text-ink-5 text-sm">({{ checkedChangedCount }}/{{ changedPreviewItems.length }})</span>
              </label>
              <div class="flex border border-line rounded-lg overflow-hidden">
                <button
                    class="py-1.5 px-3.5 bg-transparent border-none text-sm cursor-pointer transition-[background-color,color]"
                    :class="diffViewMode === 'raw' ? 'bg-blue/[0.12] text-blue-2' : 'text-ink-5'"
                    @click="diffViewMode = 'raw'"
                >원문 보기
                </button>
                <button
                    class="py-1.5 px-3.5 bg-transparent border-none text-sm cursor-pointer transition-[background-color,color] border-l border-line"
                    :class="diffViewMode === 'diff' ? 'bg-blue/[0.12] text-blue-2' : 'text-ink-5'"
                    @click="diffViewMode = 'diff'"
                >변경사항 보기
                </button>
              </div>
            </div>

            <div class="flex flex-col gap-3">
              <div
                  v-for="item in changedPreviewItems"
                  :key="item.key"
                  class="rounded-[10px] overflow-hidden transition-[opacity,border-color] duration-200 border"
                  :class="checkedKeys.has(item.key) ? 'border-amber/30' : 'opacity-45 border-line'"
              >
                <div
                    class="flex items-center gap-2.5 py-2.5 px-3.5 border-b"
                    :class="checkedKeys.has(item.key) ? 'bg-amber/[0.05] border-amber/15' : 'bg-surface border-line'"
                >
                  <input
                      type="checkbox"
                      class="w-[15px] h-[15px] cursor-pointer accent-blue shrink-0"
                      :checked="checkedKeys.has(item.key)"
                      @change="toggleItem(item.key)"
                  />
                  <span class="text-sm text-ink-2 font-semibold">
                    {{ item.grade }}학년 {{ item.class_num }}반 {{ item.number }}번
                    <template v-if="item.student_name"> · {{ item.student_name }}</template>
                  </span>
                  <span class="text-ink-5 text-sm">|</span>
                  <span class="text-sm text-amber">{{ item.activity_name }}</span>
                </div>
                <div class="grid grid-cols-2">
                  <div class="py-3 px-3.5">
                    <div class="text-[11px] font-semibold text-ink-5 uppercase tracking-[0.06em] mb-1.5">기존</div>
                    <div class="text-sm text-ink-2 leading-relaxed whitespace-pre-wrap break-all">
                      <DiffView v-if="diffViewMode === 'diff'" :before="item.new_content" :after="item.existing_content"/>
                      <template v-else>{{ item.existing_content }}</template>
                    </div>
                  </div>
                  <div class="py-3 px-3.5 border-l border-line">
                    <div class="text-[11px] font-semibold text-ink-5 uppercase tracking-[0.06em] mb-1.5">변경 후</div>
                    <div class="text-sm text-ink-2 leading-relaxed whitespace-pre-wrap break-all">
                      <DiffView v-if="diffViewMode === 'diff'" :before="item.existing_content" :after="item.new_content"/>
                      <template v-else>{{ item.new_content }}</template>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <p v-if="newPreviewItemsCount > 0" class="text-sm text-green mt-3.5">
              + 신규 기록 {{ newPreviewItemsCount }}건은 자동으로 추가됩니다.
            </p>
            <p v-if="unchangedCount > 0" class="text-sm text-ink-5 mt-3.5">
              {{ unchangedCount }}건은 기존과 동일하여 건너뜁니다.
            </p>
          </template>
        </template>
      </div>

      <!-- Step 6: 실행 -->
      <div v-else-if="step === 6">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 6. 가져오기 실행</h3>

        <div v-if="!importResult">
          <div class="border border-line rounded-[10px] overflow-hidden mb-6">
            <div class="grid gap-3 py-[11px] px-4 border-b border-line/70 last:border-b-0 grid-detail-140">
              <span class="text-sm text-ink-5">파일</span>
              <span class="text-sm text-ink-2">{{ fileName }}</span>
            </div>
            <div class="grid gap-3 py-[11px] px-4 border-b border-line/70 last:border-b-0 grid-detail-140">
              <span class="text-sm text-ink-5">양식</span>
              <span class="text-sm text-ink-2">{{ fileType === 'A' ? 'A타입 — 행 단위 활동 형식' : 'B타입 — 열 단위 활동 형식' }}</span>
            </div>
            <div class="grid gap-3 py-[11px] px-4 border-b border-line/70 last:border-b-0 grid-detail-140">
              <span class="text-sm text-ink-5">데이터 행</span>
              <span class="text-sm text-ink-2">{{ rawData.length }}행</span>
            </div>
            <div class="grid gap-3 py-[11px] px-4 border-b border-line/70 last:border-b-0 grid-detail-140">
              <span class="text-sm text-ink-5">활동 수</span>
              <span class="text-sm text-ink-2">{{ extractedActivities.length }}개</span>
            </div>
            <div class="grid gap-3 py-[11px] px-4 grid-detail-140">
              <span class="text-sm text-ink-5">새로 만들 활동</span>
              <span class="text-sm text-ink-2">{{ Object.values(activityMatchMap).filter(v => v === 0).length }}개</span>
            </div>
          </div>

          <p v-if="importError" class="text-sm text-red my-3 mb-6">{{ importError }}</p>

          <button
              class="py-2.5 px-7 bg-blue/[0.15] border border-blue/40 rounded-lg text-blue-2 text-base font-semibold cursor-pointer transition-colors enabled:hover:bg-blue/25 disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="importing"
              @click="doImport"
          >
            {{ importing ? '가져오는 중...' : '가져오기 실행' }}
          </button>
        </div>
      </div>

      <!-- 완료 -->
      <div v-else-if="step === 7" class="flex flex-col items-center gap-4 py-12">
        <div class="text-[40px] text-green">✓</div>
        <p class="text-xl font-bold text-ink m-0">가져오기 완료</p>
        <div class="flex gap-8">
          <div class="flex flex-col items-center gap-1">
            <span class="text-[28px] font-bold text-blue-2">{{ importResult.students_created }}</span>
            <span class="text-sm text-ink-5">학생 신규 생성</span>
          </div>
          <div class="flex flex-col items-center gap-1">
            <span class="text-[28px] font-bold text-blue-2">{{ importResult.students_updated }}</span>
            <span class="text-sm text-ink-5">학생 업데이트</span>
          </div>
          <div class="flex flex-col items-center gap-1">
            <span class="text-[28px] font-bold text-blue-2">{{ importResult.records_saved }}</span>
            <span class="text-sm text-ink-5">기록 저장</span>
          </div>
        </div>
        <button
            class="mt-2 py-[9px] px-6 bg-transparent border border-line rounded-lg text-ink-5 text-base cursor-pointer transition-colors hover:bg-line hover:text-ink-3"
            @click="resetWizard"
        >새로 가져오기</button>
      </div>

      <!-- Step 3 하단 파일 미리보기 -->
      <div v-if="step === 3 && rawHeaders.length > 0">
        <div class="mb-3 pb-3 border-b border-line"></div>
        <div class="border border-line rounded-[10px] overflow-hidden mt-8 mb-2">
          <div class="flex items-center justify-between py-[9px] px-3.5 bg-surface cursor-pointer select-none"
               @click="previewCollapsed = !previewCollapsed">
            <span class="text-sm text-ink-5">{{ fileName }} · {{ rawData.length }}행</span>
            <span class="text-sm text-ink-5">{{ previewCollapsed ? '펼치기 ▾' : '접기 ▴' }}</span>
          </div>
          <div v-if="!previewCollapsed" class="overflow-x-auto border-t border-line">
            <table class="border-collapse min-w-full text-sm">
              <thead><tr>
                <th v-for="h in rawHeaders" :key="h" class="py-2 px-3 bg-surface text-ink-5 font-semibold text-left whitespace-nowrap border-b border-line">{{ h }}</th>
              </tr></thead>
              <tbody>
              <tr v-for="(row, i) in previewRows" :key="i">
                <td v-for="(cell, j) in row" :key="j" class="py-[7px] px-3 text-ink-2 border-b border-line/70 whitespace-nowrap max-w-[200px] overflow-hidden text-ellipsis">{{ cell }}</td>
              </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

    </WizardLayout>

  </div>
</template>

<style scoped>
/* 학번 오류 행 — 자식 td에 색 적용 */
.sid-row-error td {
  color: var(--c-red);
}

/* details 토글 화살표 — ::after pseudo-element 필수 */
.sample-section-summary::after {
  content: '▶';
  font-size: 11px;
  color: var(--c-ink-5);
  transition: transform 0.2s;
  flex-shrink: 0;
  margin-left: 6px;
}

details[open] .sample-section-summary::after {
  transform: rotate(90deg);
}

details:not([open]) .sample-section-toggle-label::after {
  content: '펼치기';
}

details[open] .sample-section-toggle-label::after {
  content: '접기';
}
</style>
