<script setup>
import {computed, ref, watch} from 'vue'
import {ChevronDown, ChevronRight, Download, FileSpreadsheet, Users} from 'lucide-vue-next'
import {Workbook} from 'exceljs'
import {save} from '@tauri-apps/plugin-dialog'
import BaseModal from './BaseModal.vue'
import {useFileStore} from '../stores/file.js'
import {useStudentStore} from '../stores/student.js'
import {SAMPLE_CSV} from '../data/sampleStudentCsv.ts'

const props = defineProps({
  area: {type: Object, required: true},
  allStudents: {type: Array, default: () => []},
  initialStudentIds: {type: Array, default: () => []},
})

const emit = defineEmits(['close', 'saved'])

const fileStore = useFileStore()
const studentStore = useStudentStore()

// ── 뷰 상태 ──────────────────────────────────────────────
const currentView = ref('list') // 'list' | 'excel'

// ── 학생 선택 상태 ────────────────────────────────────────
const selectedIds = ref(new Set())
const expandedGroups = ref(new Set())

watch(
    () => props.initialStudentIds,
    (ids) => { selectedIds.value = new Set(ids) },
    {immediate: true}
)

const groups = computed(() => {
  const map = new Map()
  for (const s of props.allStudents) {
    const key = `${s.grade}-${s.class_num}`
    if (!map.has(key)) {
      map.set(key, {grade: s.grade, classNum: s.class_num, students: []})
    }
    map.get(key).students.push(s)
  }
  return [...map.values()]
})

function isGroupExpanded(key) { return expandedGroups.value.has(key) }

function toggleGroup(key) {
  const next = new Set(expandedGroups.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expandedGroups.value = next
}

function groupKey(g) { return `${g.grade}-${g.classNum}` }
function isGroupAllSelected(g) { return g.students.every(s => selectedIds.value.has(s.id)) }
function isGroupPartialSelected(g) {
  const count = g.students.filter(s => selectedIds.value.has(s.id)).length
  return count > 0 && count < g.students.length
}

function toggleGroupAll(g) {
  const next = new Set(selectedIds.value)
  if (isGroupAllSelected(g)) g.students.forEach(s => next.delete(s.id))
  else g.students.forEach(s => next.add(s.id))
  selectedIds.value = next
}

function toggleStudent(id) {
  const next = new Set(selectedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedIds.value = next
}

// ── 서버 에러 ─────────────────────────────────────────────
const serverError = ref('')
function setServerError(msg) { serverError.value = msg }
defineExpose({setServerError})

function submit() {
  serverError.value = ''
  emit('saved', [...selectedIds.value])
}

// ── 엑셀 뷰 ──────────────────────────────────────────────
const fileInputRef = ref(null)
const dragging = ref(false)
const excelError = ref('')
const excelStatus = ref(null) // { selected: N, newlyAdded: M } | null
const parsing = ref(false)

const COL_ALIASES = {
  grade: ['학년', 'grade'],
  classNum: ['반', 'class', '학급', '반번호', 'classnum', 'class_num'],
  number: ['번호', 'number', 'num', '번', '출석번호'],
  name: ['이름', 'name', '성명', '학생명', '학생이름'],
}

function openExcelView() {
  excelError.value = ''
  excelStatus.value = null
  currentView.value = 'excel'
}

function backToList() {
  currentView.value = 'list'
  excelError.value = ''
}

function onDragOver(e) { e.preventDefault(); dragging.value = true }
function onDragLeave() { dragging.value = false }
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
    for (const row of csvRows) worksheet.addRow(row)
    const buffer = await workbook.xlsx.writeBuffer()
    await fileStore.writeBytesFile(path, bufferToBase64(buffer))
  } catch (e) {
    excelError.value = '샘플 파일 저장 실패: ' + String(e)
  }
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

function decodeCSVBytes(buffer) {
  const bytes = new Uint8Array(buffer)
  if (bytes[0] === 0xEF && bytes[1] === 0xBB && bytes[2] === 0xBF)
    return new TextDecoder('utf-8').decode(bytes.subarray(3))
  for (const enc of ['utf-8', 'euc-kr'])
    try { return new TextDecoder(enc, {fatal: true}).decode(bytes) } catch {}
  return new TextDecoder('utf-8').decode(bytes)
}

function autoDetectColMap(headers) {
  const map = {grade: null, classNum: null, number: null, name: null}
  headers.forEach((header, idx) => {
    const h = header.toLowerCase().replace(/\s/g, '')
    for (const [field, aliases] of Object.entries(COL_ALIASES)) {
      if (map[field] === null && aliases.includes(h)) map[field] = idx
    }
  })
  return map
}

async function processFile(file) {
  excelError.value = ''
  parsing.value = true

  const ext = file.name.split('.').pop().toLowerCase()
  if (!['csv', 'xlsx'].includes(ext)) {
    excelError.value = 'CSV(.csv) 또는 엑셀(.xlsx) 파일만 지원합니다.'
    parsing.value = false
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
        excelError.value = '데이터가 없습니다. 헤더 행 포함 최소 2행이 필요합니다.'
        parsing.value = false
        return
      }

      const headers = rows[0].map(h => String(h ?? '').trim())
      const colMap = autoDetectColMap(headers)

      const missing = Object.entries(colMap)
          .filter(([, v]) => v === null)
          .map(([k]) => ({grade: '학년', classNum: '반', number: '번호', name: '이름'}[k]))

      if (missing.length > 0) {
        excelError.value = `열 자동 감지 실패: [${missing.join(', ')}] 열을 찾을 수 없습니다. 샘플 파일 양식을 확인해 주세요.`
        parsing.value = false
        return
      }

      const {grade: gi, classNum: ci, number: ni, name: nmi} = colMap
      const parsedRows = rows.slice(1)
          .map(row => ({
            grade: Number(row[gi]),
            classNum: Number(row[ci]),
            number: Number(row[ni]),
            name: String(row[nmi] ?? '').trim(),
          }))
          .filter(r => r.grade >= 1 && r.classNum >= 1 && r.number >= 1 && r.name)

      if (parsedRows.length === 0) {
        excelError.value = '유효한 학생 데이터가 없습니다. 학년·반·번호·이름을 모두 확인해 주세요.'
        parsing.value = false
        return
      }

      // 학생 일괄 upsert (없으면 추가, 있으면 유지)
      const {inserted} = await studentStore.bulkUpsertStudents(
          parsedRows.map(r => ({grade: r.grade, class_num: r.classNum, number: r.number, name: r.name}))
      )
      await studentStore.fetchStudents()

      // fetchStudents 후 갱신된 store에서 매칭 (allStudents prop이 reactive로 자동 반영됨)
      const lookupKey = (grade, classNum, number) => `${grade}-${classNum}-${number}`
      const targetKeys = new Set(parsedRows.map(r => lookupKey(r.grade, r.classNum, r.number)))

      const matchedIds = new Set()
      for (const s of studentStore.students) {
        if (targetKeys.has(lookupKey(s.grade, s.class_num, s.number))) {
          matchedIds.add(s.id)
        }
      }

      selectedIds.value = matchedIds
      excelStatus.value = {selected: matchedIds.size, newlyAdded: inserted}
      currentView.value = 'list'
    } catch (err) {
      excelError.value = '파일 파싱 중 오류가 발생했습니다: ' + err.message
    } finally {
      parsing.value = false
    }
  }
  reader.readAsArrayBuffer(file)
}
</script>

<template>
  <BaseModal
      title="학생 배정"
      :label="area.name"
      max-width="640px"
      max-height="80vh"
      @close="emit('close')"
  >
    <!-- ── 리스트 뷰 바디 ─────────────────────────────────── -->
    <div v-if="currentView === 'list'" class="flex-1 overflow-y-auto py-3">
      <div v-if="excelStatus"
           class="flex items-center gap-1.5 mx-5 mb-1 px-3 py-2 bg-green/[8%] border border-green/20 rounded-lg text-sm">
        <span class="text-green font-semibold">{{ excelStatus.selected }}명 선택됨</span>
        <span v-if="excelStatus.newlyAdded > 0" class="text-blue-2">
          · {{ excelStatus.newlyAdded }}명 신규 추가됨
        </span>
      </div>

      <p v-if="allStudents.length === 0" class="text-base text-ink-5 leading-[1.7] px-6 py-6 m-0">
        등록된 학생이 없습니다.<br>학생 관리에서 먼저 추가하세요.
      </p>

      <div v-else class="flex flex-col">
        <div v-for="g in groups" :key="groupKey(g)" class="border-b border-line last:border-b-0">
          <div
              class="flex items-center justify-between px-5 py-3 cursor-pointer transition-colors select-none hover:bg-blue/[5%]"
              @click="toggleGroup(groupKey(g))"
          >
            <div class="flex items-center gap-2.5">
              <input
                  type="checkbox"
                  class="w-4 h-4 cursor-pointer accent-blue shrink-0"
                  :checked="isGroupAllSelected(g)"
                  :indeterminate="isGroupPartialSelected(g)"
                  @change.stop="toggleGroupAll(g)"
                  @click.stop
              />
              <span class="text-base font-semibold text-ink-2">{{ g.grade }}학년 {{ g.classNum }}반</span>
              <span class="text-sm text-ink-4">{{ g.students.length }}명</span>
              <span v-if="isGroupPartialSelected(g) || isGroupAllSelected(g)"
                    class="text-sm text-blue-2 bg-blue/[12%] rounded px-1.5 py-px">
                {{ g.students.filter(s => selectedIds.has(s.id)).length }}명 선택
              </span>
            </div>
            <ChevronDown v-if="isGroupExpanded(groupKey(g))" :size="16" class="text-ink-4 shrink-0"/>
            <ChevronRight v-else :size="16" class="text-ink-4 shrink-0"/>
          </div>

          <div v-if="isGroupExpanded(groupKey(g))" class="flex flex-col py-1 pb-2 pl-5 bg-base/40">
            <label v-for="s in g.students" :key="s.id"
                   class="flex items-center gap-2.5 py-2 pr-5 cursor-pointer transition-colors hover:bg-blue/[4%]">
              <input
                  type="checkbox"
                  class="w-[15px] h-[15px] cursor-pointer accent-blue shrink-0"
                  :checked="selectedIds.has(s.id)"
                  @change="toggleStudent(s.id)"
              />
              <span class="text-sm text-ink-4 w-9 shrink-0">{{ s.number }}번</span>
              <span class="text-base text-ink-2">{{ s.name }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>

    <!-- ── 엑셀 뷰 바디 ──────────────────────────────────── -->
    <div v-else class="flex-1 overflow-y-auto pt-5 px-6 pb-2 flex flex-col gap-4">
      <input
          ref="fileInputRef"
          type="file"
          accept=".csv,.xlsx"
          class="hidden"
          @change="onFileChange"
      />

      <div class="flex items-center justify-between gap-3">
        <div class="text-base text-ink-5 leading-relaxed">
          <p class="m-0">학생 명단이 담긴 CSV 또는 엑셀 파일을 업로드해 주세요.</p>
          <p class="m-0">파일에 <strong class="text-ink-2">학년, 반, 번호, 이름</strong> 열이 포함되어야 합니다.</p>
        </div>
        <button
            class="flex items-center gap-1.5 py-[7px] px-3 rounded-lg border border-blue/30 bg-blue/[8%] text-blue-2 text-sm cursor-pointer whitespace-nowrap shrink-0 transition-colors hover:bg-blue/15"
            @click="downloadSample"
        >
          <Download :size="14"/>
          샘플 파일 다운로드
        </button>
      </div>

      <div class="text-sm text-ink-5 bg-blue/[6%] border border-blue/20 rounded-lg px-3.5 py-2.5 leading-[1.7]">
        엑셀 파일에 담긴 학생을 <strong class="text-ink-2">{{ area.name }}</strong> 영역에 일괄 배정합니다.
        <span class="text-amber">엑셀 파일 명단에 없는 학생은 이 영역에서 배정 취소됩니다.</span>
      </div>

      <div
          class="border-2 border-dashed rounded-[14px] py-[52px] px-6 flex flex-col items-center gap-2.5 transition-colors"
          :class="parsing
            ? 'cursor-default border-green/30 bg-green/[3%]'
            : dragging
              ? 'border-blue/50 bg-blue/[4%] cursor-pointer'
              : 'border-line cursor-pointer hover:border-blue/50 hover:bg-blue/[4%]'"
          @dragover="onDragOver"
          @dragleave="onDragLeave"
          @drop="onDrop"
          @click="!parsing && fileInputRef.click()"
      >
        <FileSpreadsheet v-if="!parsing" :size="36" class="text-ink-5"/>
        <Users v-else :size="36" class="text-green opacity-70"/>
        <p class="text-base text-ink-5 m-0">
          <template v-if="parsing">학생 명단을 분석하는 중...</template>
          <template v-else>
            파일을 여기에 드래그하거나 <span class="text-blue-2 underline">파일 선택</span>
          </template>
        </p>
        <p v-if="!parsing" class="text-sm text-ink-5 m-0">CSV, XLSX 지원</p>
      </div>

      <div v-if="excelError"
           class="text-base text-red/80 bg-red/[8%] border border-red/20 rounded-lg px-3.5 py-2.5 leading-relaxed">
        {{ excelError }}
      </div>
    </div>

    <!-- ── 푸터 ──────────────────────────────────────────── -->
    <template #footer>
      <span class="text-base text-ink-5">{{ selectedIds.size }}명 선택됨</span>
      <div class="flex gap-2 items-center">
        <template v-if="currentView === 'list'">
          <p v-if="serverError" class="text-sm text-red m-0 mr-3">{{ serverError }}</p>
          <button class="btn-secondary flex items-center gap-1.5" @click="openExcelView">
            <FileSpreadsheet :size="14"/>
            엑셀로 일괄배정
          </button>
          <button class="btn-secondary" @click="emit('close')">취소</button>
          <button class="btn-primary" @click="submit">저장</button>
        </template>
        <template v-else>
          <button class="btn-secondary" :disabled="parsing" @click="backToList">
            돌아가기
          </button>
        </template>
      </div>
    </template>
  </BaseModal>
</template>
