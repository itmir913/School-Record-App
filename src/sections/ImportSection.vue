<script setup>
import {computed, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'
import {ArrowLeft, ArrowRight, Download, FileSpreadsheet} from 'lucide-vue-next'
import DiffView from '../components/DiffView.vue'
import {Workbook} from 'exceljs'

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
const wizardBodyRef = ref(null)
const idMode = ref('fields') // 'fields' | 'studentId'

watch(step, () => {
  wizardBodyRef.value?.scrollTo({top: 0, behavior: 'smooth'})
})
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
  const vals = Object.values(colMap.value).filter(v => v !== null)
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

  parseError.value = ''
  previewError.value = ''
  importError.value = ''

  if (step.value === 3) {
    await loadDbActivities()
    initActivityMatchMap()
  }
  if (step.value === 4) {
    await loadPreview()
  }
  step.value++
}

function goPrev() {
  if (step.value > 1) step.value--
}

async function loadDbActivities() {
  try {
    const acts = await invoke('get_activities')
    dbActivities.value = acts.map(a => ({id: a.id, name: a.name}))
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
      backendItems = await invoke('preview_import_records', {
        records: existingActRecords.map(r => ({
          grade: r.grade, class_num: r.class_num, number: r.number,
          name: r.name, activity_id: r.activity_id, content: r.content,
        }))
      })
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
        finalMap[name] = await invoke('create_activity', {name})
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

    importResult.value = await invoke('bulk_import_records', {records})
    step.value++
  } catch (e) {
    importError.value = '가져오기 실패: ' + String(e)
  } finally {
    importing.value = false
  }
}

// ── 예시 파일 다운로드 ─────────────────────────────────────────

async function downloadSampleA() {
  const rows = [
    {학년: 3, 반: 1, 번호: 1, 이름: '학생A', 활동명: '현장체험학습', 활동내용: '지역 기관을 탐방하며 사회적 기능을 이해하고 성실히 활동에 참여함.'},
    {학년: 3, 반: 1, 번호: 1, 이름: '학생A', 활동명: '학급자치회', 활동내용: '회의에 적극 참여하여 의견을 제시하고 의사결정에 기여함.'},
    {학년: 3, 반: 1, 번호: 1, 이름: '학생A', 활동명: '전교학생회', 활동내용: '학교 행사 기획에 참여하며 책임감 있는 태도를 보임.'},
    {학년: 3, 반: 1, 번호: 1, 이름: '학생A', 활동명: '체육대회', 활동내용: '경기와 응원에 적극 참여하며 협동심을 발휘함.'},
    {학년: 3, 반: 1, 번호: 1, 이름: '학생A', 활동명: '독서토론', 활동내용: '주제에 대해 논리적으로 의견을 제시하며 토론에 참여함.'},
    {학년: 3, 반: 1, 번호: 1, 이름: '학생A', 활동명: '실험실 안전교육', 활동내용: '안전 수칙을 정확히 이해하고 이를 성실히 준수함.'},
    {학년: 3, 반: 1, 번호: 1, 이름: '학생A', 활동명: '재난 대비 훈련', 활동내용: '재난 대비 훈련 절차를 숙지하고 신속하게 대응하는 태도를 보임.'},
    {학년: 3, 반: 1, 번호: 2, 이름: '학생B', 활동명: '체육대회', 활동내용: '다양한 종목에 참여하며 규칙을 준수하는 태도를 보임.'},
    {학년: 3, 반: 1, 번호: 3, 이름: '학생C', 활동명: '독서토론', 활동내용: '도서를 바탕으로 자신의 생각을 논리적으로 표현함.'},
    {학년: 3, 반: 1, 번호: 3, 이름: '학생C', 활동명: '실험실 안전교육', 활동내용: '실험 전 안전 점검을 철저히 하며 책임감 있게 참여함.'},
    {학년: 3, 반: 1, 번호: 4, 이름: '학생D', 활동명: '현장체험학습', 활동내용: '체험 활동을 통해 다양한 직업 세계를 이해함.'},
    {학년: 3, 반: 1, 번호: 4, 이름: '학생D', 활동명: '재난 대비 훈련', 활동내용: '재난 상황별 행동 요령을 숙지하고 훈련에 참여함.'},
    {학년: 3, 반: 1, 번호: 5, 이름: '학생E', 활동명: '학급자치회', 활동내용: '학급 행사 준비 과정에서 협력적인 태도를 보이며 역할을 수행함.'},
    {학년: 3, 반: 1, 번호: 6, 이름: '학생F', 활동명: '체육대회', 활동내용: '팀원들과 협력하여 경기에 성실히 참여함.'},
    {학년: 3, 반: 1, 번호: 6, 이름: '학생F', 활동명: '전교학생회', 활동내용: '학생 의견을 수렴하여 학교 운영에 기여함.'},
    {학년: 3, 반: 1, 번호: 6, 이름: '학생F', 활동명: '재난 대비 훈련', 활동내용: '훈련에 적극 참여하며 위기 대응 능력을 기름.'},
    {학년: 3, 반: 1, 번호: 7, 이름: '학생G', 활동명: '현장체험학습', 활동내용: '현장 경험을 통해 배운 내용을 바탕으로 이해를 심화함.'},
    {학년: 3, 반: 1, 번호: 7, 이름: '학생G', 활동명: '독서토론', 활동내용: '다양한 관점에서 의견을 제시하며 토론에 참여함.'},
    {학년: 3, 반: 1, 번호: 7, 이름: '학생G', 활동명: '실험실 안전교육', 활동내용: '안전 수칙을 준수하며 성실히 교육에 참여함.'},
    {학년: 3, 반: 1, 번호: 8, 이름: '학생H', 활동명: '재난 대비 훈련', 활동내용: '훈련 절차를 성실히 따르며 안전 의식을 함양함.'},
  ]
  const filePath = await save({
    title: 'A타입 예시 파일 저장',
    defaultPath: 'A타입_예시.xlsx',
    filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
  })
  if (!filePath) return

  const headers = ['학년', '반', '번호', '이름', '활동명', '활동내용']
  const workbook = new Workbook()
  const worksheet = workbook.addWorksheet('예시')
  worksheet.addRow(headers)
  for (const row of rows) {
    worksheet.addRow(headers.map(h => row[h]))
  }
  const buffer = await workbook.xlsx.writeBuffer()
  const data = bufferToBase64(buffer)
  await invoke('write_bytes_file', {path: filePath, data})
}

async function downloadSampleB() {
  const cols = ['학년', '반', '번호', '이름', '현장체험학습', '학급자치회', '전교학생회', '체육대회', '독서토론', '실험실 안전교육', '재난 대비 훈련']
  const empty = () => Object.fromEntries(cols.map(c => [c, '']))
  const rows = [
    {
      ...empty(),
      학년: 3,
      반: 1,
      번호: 1,
      이름: '학생A',
      현장체험학습: '지역 기관을 탐방하며 사회적 기능을 이해하고 성실히 활동에 참여함.',
      학급자치회: '회의에 적극 참여하여 의견을 제시하고 의사결정에 기여함.',
      전교학생회: '학교 행사 기획에 참여하며 책임감 있는 태도를 보임.',
      체육대회: '경기와 응원에 적극 참여하며 협동심을 발휘함.',
      독서토론: '주제에 대해 논리적으로 의견을 제시하며 토론에 참여함.',
      '실험실 안전교육': '안전 수칙을 정확히 이해하고 이를 성실히 준수함.',
      '재난 대비 훈련': '재난 대비 훈련 절차를 숙지하고 신속하게 대응하는 태도를 보임.'
    },
    {...empty(), 학년: 3, 반: 1, 번호: 2, 이름: '학생B', 체육대회: '다양한 종목에 참여하며 규칙을 준수하는 태도를 보임.'},
    {
      ...empty(),
      학년: 3,
      반: 1,
      번호: 3,
      이름: '학생C',
      독서토론: '도서를 바탕으로 자신의 생각을 논리적으로 표현함.',
      '실험실 안전교육': '실험 전 안전 점검을 철저히 하며 책임감 있게 참여함.'
    },
    {
      ...empty(),
      학년: 3,
      반: 1,
      번호: 4,
      이름: '학생D',
      현장체험학습: '체험 활동을 통해 다양한 직업 세계를 이해함.',
      '재난 대비 훈련': '재난 상황별 행동 요령을 숙지하고 훈련에 참여함.'
    },
    {...empty(), 학년: 3, 반: 1, 번호: 5, 이름: '학생E', 학급자치회: '학급 행사 준비 과정에서 협력적인 태도를 보이며 역할을 수행함.'},
    {
      ...empty(),
      학년: 3,
      반: 1,
      번호: 6,
      이름: '학생F',
      전교학생회: '학생 의견을 수렴하여 학교 운영에 기여함.',
      체육대회: '팀원들과 협력하여 경기에 성실히 참여함.',
      '재난 대비 훈련': '훈련에 적극 참여하며 위기 대응 능력을 기름.'
    },
    {
      ...empty(),
      학년: 3,
      반: 1,
      번호: 7,
      이름: '학생G',
      현장체험학습: '현장 경험을 통해 배운 내용을 바탕으로 이해를 심화함.',
      독서토론: '다양한 관점에서 의견을 제시하며 토론에 참여함.',
      '실험실 안전교육': '안전 수칙을 준수하며 성실히 교육에 참여함.'
    },
    {...empty(), 학년: 3, 반: 1, 번호: 8, 이름: '학생H', '재난 대비 훈련': '훈련 절차를 성실히 따르며 안전 의식을 함양함.'},
  ]
  const filePath = await save({
    title: 'B타입 예시 파일 저장',
    defaultPath: 'B타입_예시.xlsx',
    filters: [{name: 'Excel 파일', extensions: ['xlsx']}],
  })
  if (!filePath) return

  const workbook = new Workbook()
  const worksheet = workbook.addWorksheet('예시')
  worksheet.addRow(cols)
  for (const row of rows) {
    worksheet.addRow(cols.map(c => row[c]))
  }
  const buffer = await workbook.xlsx.writeBuffer()
  const data = bufferToBase64(buffer)
  await invoke('write_bytes_file', {path: filePath, data})
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
  previewLoading.value = false
  previewError.value = ''
  previewItems.value = []
  checkedKeys.value = new Set()
  diffViewMode.value = 'raw'
  pendingRecords.value = []
}
</script>

<template>
  <div class="section">

    <!-- 툴바 -->
    <div class="toolbar">
      <div class="section-header">
        <h2 class="section-title">데이터 가져오기(Import)</h2>
        <p class="section-desc">다양한 형식의 학교생활기록부 기재 문장을 본 프로그램으로 가져옵니다.</p>
      </div>

      <div class="step-indicator">
        <span
            v-for="n in 6"
            :key="n"
            class="step-dot"
            :class="{ 'step-dot--active': step === n, 'step-dot--done': step > n }"
        >{{ n }}</span>
      </div>
    </div>

    <!-- 본문 -->
    <div class="wizard-body" ref="wizardBodyRef">

      <!-- Step 1: 파일 업로드 -->
      <div v-if="step === 1" class="step-content">

        <!-- 예시 파일 다운로드 -->
        <div class="sample-section">
          <h3 class="step-title">Step 0. 가져오기(Import) 가능한 파일 안내</h3>
          <p class="step-desc">가져오기 가능한 파일은 두 가지 유형(행 단위, 열 단위)입니다. 예시 다운로드 버튼을 눌러 각 유형의 예시를 확인하세요.</p>

          <div class="type-cards">
            <div class="type-card sample-type-card">
              <div class="type-card-top">
                <span class="type-badge type-badge--a">A 타입</span>
                <span class="type-name">행 단위 활동 형식</span>
              </div>
              <p class="type-desc">한 행에 학생 1명의 활동 1개를 기재합니다.<br>학생 1명의 활동은 여러 행에 걸쳐 기록됩니다(학생 1명 = 여러 행).</p>
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
                    <td>학생A</td>
                    <td>현장체험학습</td>
                    <td>지역 기관을 탐방...</td>
                  </tr>
                  <tr>
                    <td>3</td>
                    <td>1</td>
                    <td>1</td>
                    <td>학생A</td>
                    <td>학급자치회</td>
                    <td>회의에 적극 참여...</td>
                  </tr>
                  <tr>
                    <td>3</td>
                    <td>1</td>
                    <td>2</td>
                    <td>학생B</td>
                    <td>체육대회</td>
                    <td>다양한 종목에 참여...</td>
                  </tr>
                  </tbody>
                </table>
              </div>
              <button class="btn-sample-dl" @click.stop="downloadSampleA">
                <Download :size="13"/>
                A타입 예시 다운로드
              </button>
            </div>

            <div class="type-card sample-type-card">
              <div class="type-card-top">
                <span class="type-badge type-badge--b">B 타입</span>
                <span class="type-name">열 단위 활동 형식</span>
              </div>
              <p class="type-desc">활동이 열(헤더)로 구분된 형식입니다.<br>학생 1명의 모든 활동이 한 행에 기록됩니다(학생 1명 = 1행).</p>
              <div class="sample-table-wrap">
                <table class="sample-table">
                  <thead>
                  <tr>
                    <th>학년</th>
                    <th>반</th>
                    <th>번호</th>
                    <th>이름</th>
                    <th>현장체험학습</th>
                    <th>학급자치회</th>
                  </tr>
                  </thead>
                  <tbody>
                  <tr>
                    <td>3</td>
                    <td>1</td>
                    <td>1</td>
                    <td>학생A</td>
                    <td>지역 기관을...</td>
                    <td>회의에 적극...</td>
                  </tr>
                  <tr>
                    <td>3</td>
                    <td>1</td>
                    <td>2</td>
                    <td>학생B</td>
                    <td></td>
                    <td></td>
                  </tr>
                  <tr>
                    <td>3</td>
                    <td>1</td>
                    <td>5</td>
                    <td>학생E</td>
                    <td></td>
                    <td>학급 행사 준비...</td>
                  </tr>
                  </tbody>
                </table>
              </div>
              <button class="btn-sample-dl" @click.stop="downloadSampleB">
                <Download :size="13"/>
                B타입 예시 다운로드
              </button>
            </div>
          </div>
        </div>

        <div class="border-hr"></div>

        <h3 class="step-title">Step 1. 가져올 파일 선택</h3>
        <p class="step-desc">CSV 또는 XLSX 파일을 선택하거나 드래그하세요.</p>

        <input ref="fileInputRef" type="file" accept=".csv,.xlsx,.xls" style="display:none" @change="onFileChange"/>

        <div
            class="drop-zone"
            :class="{ 'drop-zone--dragging': dragging, 'drop-zone--loaded': rawHeaders.length > 0 }"
            @dragover="onDragOver"
            @dragleave="onDragLeave"
            @drop="onDrop"
            @click="fileInputRef.click()"
        >
          <FileSpreadsheet :size="40" class="drop-icon"/>
          <p v-if="!fileName" class="drop-main">파일을 여기에 드래그하거나 클릭하여 선택</p>
          <p v-else class="drop-main drop-main--loaded">{{ fileName }}</p>
          <p class="drop-hint">CSV, XLSX, XLS 지원</p>
        </div>

        <p v-if="parseError" class="error-text">{{ parseError }}</p>

        <div v-if="rawHeaders.length > 0" class="preview-block">

          <div class="border-hr"></div>

          <p class="preview-label">미리보기 (첫 {{ Math.min(rawData.length, 5) }}행)</p>
          <div class="preview-table-wrap">
            <table class="preview-table">
              <thead>
              <tr>
                <th v-for="h in rawHeaders" :key="h">{{ h }}</th>
              </tr>
              </thead>
              <tbody>
              <tr v-for="(row, i) in previewRows" :key="i">
                <td v-for="(cell, j) in row" :key="j">{{ cell }}</td>
              </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- Step 2: 양식 선택 -->
      <div v-else-if="step === 2" class="step-content">
        <h3 class="step-title">Step 2. 엑셀 파일 양식 선택</h3>
        <p class="step-desc">
          가져올 엑셀 파일 양식에 맞는 타입을 선택하세요.
          잘못된 타입을 선택하면 데이터 오류 또는 항목 누락이 발생할 수 있습니다.
        </p>

        <div class="type-cards">
          <div
              class="type-card"
              :class="{ 'type-card--selected': fileType === 'A' }"
              @click="fileType = 'A'"
          >
            <div class="type-card-top">
              <span class="type-badge type-badge--a">A 타입</span>
              <span class="type-name">행 단위 활동 형식</span>
            </div>
            <p class="type-desc">한 행에 학생 1명의 활동 1개를 기재합니다.<br>학생 1명의 활동은 여러 행에 걸쳐 기록됩니다(학생 1명 = 여러 행).</p>
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
                  <td>학생A</td>
                  <td>현장체험학습</td>
                  <td>지역 기관을 탐방...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>1</td>
                  <td>학생A</td>
                  <td>학급자치회</td>
                  <td>회의에 적극 참여...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>2</td>
                  <td>학생B</td>
                  <td>체육대회</td>
                  <td>다양한 종목에 참여...</td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div
              class="type-card"
              :class="{ 'type-card--selected': fileType === 'B' }"
              @click="fileType = 'B'"
          >
            <div class="type-card-top">
              <span class="type-badge type-badge--b">B 타입</span>
              <span class="type-name">열 단위 활동 형식</span>
            </div>
            <p class="type-desc">활동이 열(헤더)로 구분된 형식입니다.<br>학생 1명의 모든 활동이 한 행에 기록됩니다(학생 1명 = 1행).</p>
            <div class="sample-table-wrap">
              <table class="sample-table">
                <thead>
                <tr>
                  <th>학년</th>
                  <th>반</th>
                  <th>번호</th>
                  <th>이름</th>
                  <th>현장체험학습</th>
                  <th>학급자치회</th>
                </tr>
                </thead>
                <tbody>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>1</td>
                  <td>학생A</td>
                  <td>지역 기관을...</td>
                  <td>회의에 적극...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>2</td>
                  <td>학생B</td>
                  <td></td>
                  <td></td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>5</td>
                  <td>학생E</td>
                  <td></td>
                  <td>학급 행사 준비...</td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <!-- Step 3: 열 매핑 -->
      <div v-else-if="step === 3" class="step-content">
        <h3 class="step-title">Step 3. 열 매핑</h3>
        <p class="step-desc">엑셀 파일의 각 열을 올바르게 연결하세요. <span class="required">*</span> 는 필수입니다.</p>

        <!-- 학생 식별 방식 토글 -->
        <div class="id-mode-section">
          <span class="id-mode-label">학생 식별 방식</span>
          <div class="id-mode-buttons">
            <button
                class="id-mode-btn"
                :class="{ 'id-mode-btn--active': idMode === 'fields' }"
                @click="idMode = 'fields'"
            >학년 · 반 · 번호
            </button>
            <button
                class="id-mode-btn"
                :class="{ 'id-mode-btn--active': idMode === 'studentId' }"
                @click="idMode = 'studentId'"
            >학번
            </button>
          </div>
        </div>

        <div class="col-map-list">
          <!-- 학번 모드: 단일 열 선택 + 파싱 미리보기 -->
          <template v-if="idMode === 'studentId'">
            <div class="col-map-row">
              <label class="col-map-label">학번 <span class="required">*</span></label>
              <select class="col-map-select" v-model="colMap['studentId']">
                <option :value="null">— 선택 안 함 —</option>
                <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
              </select>
            </div>
            <div v-if="colMap['studentId'] !== null" class="sid-preview">
              <p class="sid-preview-label">파싱 미리보기
                (ABCC·4자리, ABBCC·5자리, ABBCCC·6자리 / A=학년, B=반, C=번호)</p>
              <table class="preview-table sid-preview-table">
                <thead>
                <tr>
                  <th>학번 원본</th>
                  <th>학년</th>
                  <th>반</th>
                  <th>번호</th>
                </tr>
                </thead>
                <tbody>
                <tr v-for="(r, i) in studentIdPreviewRows" :key="i" :class="{ 'sid-row-error': r.error }">
                  <td>{{ r.raw }}</td>
                  <td>{{ r.grade }}</td>
                  <td>{{ r.classNum }}</td>
                  <td>{{ r.number }}</td>
                </tr>
                </tbody>
              </table>
            </div>
          </template>
          <!-- 필드 직접 지정 모드 -->
          <template v-else>
            <div v-for="field in ['grade','classNum','number']" :key="field" class="col-map-row">
              <label class="col-map-label">
                {{ FIELD_LABELS_A[field] }} <span class="required">*</span>
              </label>
              <select class="col-map-select" v-model="colMap[field]">
                <option :value="null">— 선택 안 함 —</option>
                <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
              </select>
            </div>
          </template>

          <!-- 이름 (공통, 선택) -->
          <div class="col-map-row">
            <label class="col-map-label">이름 (선택)</label>
            <select class="col-map-select" v-model="colMap['name']">
              <option :value="null">— 선택 안 함 —</option>
              <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
            </select>
          </div>

          <!-- A타입 활동 필드 -->
          <template v-if="fileType === 'A'">
            <div v-for="field in ['activityName','activityContent']" :key="field" class="col-map-row">
              <label class="col-map-label">
                {{ FIELD_LABELS_A[field] }} <span class="required">*</span>
              </label>
              <select class="col-map-select" v-model="colMap[field]">
                <option :value="null">— 선택 안 함 —</option>
                <option v-for="(h, i) in rawHeaders" :key="i" :value="i">{{ h }}</option>
              </select>
            </div>
          </template>
          <!-- B타입 활동 열 자동 표시 -->
          <template v-else>
            <div class="col-map-row">
              <label class="col-map-label">활동 열 (자동)</label>
              <div class="activity-cols-preview">
                <span v-if="extractedActivities.length === 0"
                      class="activity-cols-hint">학생 정보 열을 선택하면 나머지가 활동 열로 지정됩니다.</span>
                <span v-for="n in extractedActivities" :key="n" class="activity-col-tag">{{ n }}</span>
              </div>
            </div>
          </template>

          <p v-if="hasDuplicateCols" class="col-map-error">동일한 열을 여러 필드에 지정할 수 없습니다. 각 필드에 서로 다른 열을 선택해 주세요.</p>
        </div>
      </div>

      <!-- Step 4: 활동 매칭 -->
      <div v-else-if="step === 4" class="step-content">
        <h3 class="step-title">Step 4. 활동 매칭</h3>
        <p class="step-desc">파일의 활동명을 기존 활동에 연결하거나 새로 만드세요. 이름이 일치하면 자동 매칭됩니다.</p>

        <p v-if="extractedActivities.length === 0" class="empty-hint">파일에서 활동을 찾을 수 없습니다. 이전 단계로 돌아가 열 매핑을 확인하세요.</p>

        <div v-else class="activity-match-list">
          <div class="activity-match-header">
            <span>파일의 활동명</span>
            <span></span>
            <span>연결할 활동</span>
          </div>
          <div v-for="actName in extractedActivities" :key="actName" class="activity-match-row">
            <span class="import-act-name">{{ actName }}</span>
            <span class="match-arrow">→</span>
            <select class="activity-match-select" v-model="activityMatchMap[actName]">
              <option :value="0">＋ 새로 만들기</option>
              <option v-for="a in dbActivities" :key="a.id" :value="a.id">{{ a.name }}</option>
            </select>
          </div>
        </div>
      </div>

      <!-- Step 5: 변경사항 확인 -->
      <div v-else-if="step === 5" class="step-content">
        <h3 class="step-title">Step 5. 변경사항 확인</h3>
        <p class="step-desc">기존 데이터와 비교하여 변경될 항목을 확인하고 업데이트할 항목을 선택하세요.</p>

        <div v-if="previewLoading" class="diff-loading">기존 데이터와 비교 중...</div>
        <p v-if="previewError" class="error-text">{{ previewError }}<br><span class="error-hint">미리보기 오류가 있어도 다음 단계로 진행하면 모든 항목이 가져와집니다.</span>
        </p>

        <template v-if="!previewLoading">
          <!-- 통계 바 -->
          <div class="diff-stats-bar">
            <span class="diff-stat diff-stat--changed">변경 {{ changedPreviewItems.length }}건</span>
            <span class="diff-stat diff-stat--new">신규 {{ newPreviewItemsCount }}건</span>
            <span class="diff-stat diff-stat--same">동일 {{ unchangedCount }}건</span>
          </div>

          <!-- 변경 항목 없음 -->
          <div v-if="changedPreviewItems.length === 0" class="diff-empty">
            <p class="diff-empty-msg">변경되는 항목이 없습니다.</p>
            <p v-if="newPreviewItemsCount > 0" class="diff-empty-sub">신규 기록 {{ newPreviewItemsCount }}건이 자동으로 추가됩니다.</p>
            <p v-if="unchangedCount > 0" class="diff-empty-sub">{{ unchangedCount }}건은 기존과 동일하여 건너뜁니다.</p>
          </div>

          <!-- 변경 항목 목록 -->
          <template v-else>
            <div class="diff-controls">
              <label class="diff-select-all">
                <input
                    type="checkbox"
                    :checked="allChangedChecked"
                    @change="toggleAllChanged"
                />
                전체 선택/해제
                <span class="diff-count">({{ checkedChangedCount }}/{{ changedPreviewItems.length }})</span>
              </label>
              <div class="diff-mode-buttons">
                <button
                    class="diff-mode-btn"
                    :class="{ 'diff-mode-btn--active': diffViewMode === 'raw' }"
                    @click="diffViewMode = 'raw'"
                >원문 보기
                </button>
                <button
                    class="diff-mode-btn"
                    :class="{ 'diff-mode-btn--active': diffViewMode === 'diff' }"
                    @click="diffViewMode = 'diff'"
                >변경사항 보기
                </button>
              </div>
            </div>

            <div class="diff-list">
              <div
                  v-for="item in changedPreviewItems"
                  :key="item.key"
                  class="diff-item"
                  :class="{ 'diff-item--unchecked': !checkedKeys.has(item.key) }"
              >
                <div class="diff-item-header">
                  <input
                      type="checkbox"
                      class="diff-checkbox"
                      :checked="checkedKeys.has(item.key)"
                      @change="toggleItem(item.key)"
                  />
                  <span class="diff-student-label">
                    {{ item.grade }}학년 {{ item.class_num }}반 {{ item.number }}번
                    <template v-if="item.student_name"> · {{ item.student_name }}</template>
                  </span>
                  <span class="diff-separator">|</span>
                  <span class="diff-activity-label">{{ item.activity_name }}</span>
                </div>
                <div class="diff-boxes">
                  <div class="diff-box">
                    <div class="diff-box-label">기존</div>
                    <div class="diff-box-content">
                      <DiffView
                          v-if="diffViewMode === 'diff'"
                          :before="item.new_content"
                          :after="item.existing_content"
                      />
                      <template v-else>{{ item.existing_content }}</template>
                    </div>
                  </div>
                  <div class="diff-box diff-box--after">
                    <div class="diff-box-label">변경 후</div>
                    <div class="diff-box-content">
                      <DiffView
                          v-if="diffViewMode === 'diff'"
                          :before="item.existing_content"
                          :after="item.new_content"
                      />
                      <template v-else>{{ item.new_content }}</template>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <p v-if="newPreviewItemsCount > 0" class="diff-auto-note diff-auto-note--new">
              + 신규 기록 {{ newPreviewItemsCount }}건은 자동으로 추가됩니다.
            </p>
            <p v-if="unchangedCount > 0" class="diff-auto-note diff-auto-note--same">
              {{ unchangedCount }}건은 기존과 동일하여 건너뜁니다.
            </p>
          </template>
        </template>
      </div>

      <!-- Step 6: 실행 -->
      <div v-else-if="step === 6" class="step-content">
        <h3 class="step-title">Step 6. 가져오기 실행</h3>

        <div v-if="!importResult">
          <div class="summary-box">
            <div class="summary-row">
              <span class="summary-key">파일</span>
              <span class="summary-val">{{ fileName }}</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">양식</span>
              <span class="summary-val">{{ fileType === 'A' ? 'A타입 — 행 단위 활동 형식' : 'B타입 — 열 단위 활동 형식' }}</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">데이터 행</span>
              <span class="summary-val">{{ rawData.length }}행</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">활동 수</span>
              <span class="summary-val">{{ extractedActivities.length }}개</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">새로 만들 활동</span>
              <span class="summary-val">{{ Object.values(activityMatchMap).filter(v => v === 0).length }}개</span>
            </div>
          </div>

          <p v-if="importError" class="error-text">{{ importError }}</p>

          <button class="btn-import" :disabled="importing" @click="doImport">
            {{ importing ? '가져오는 중...' : '가져오기 실행' }}
          </button>
        </div>
      </div>

      <div v-else-if="step === 7" class="result-box">
        <div class="result-check">✓</div>
        <p class="result-title">가져오기 완료</p>
        <div class="result-stats">
          <div class="stat-item">
            <span class="stat-val">{{ importResult.students_created }}</span>
            <span class="stat-label">학생 신규 생성</span>
          </div>
          <div class="stat-item">
            <span class="stat-val">{{ importResult.students_updated }}</span>
            <span class="stat-label">학생 업데이트</span>
          </div>
          <div class="stat-item">
            <span class="stat-val">{{ importResult.records_saved }}</span>
            <span class="stat-label">기록 저장</span>
          </div>
        </div>
        <button class="btn-reset" @click="resetWizard">새로 가져오기</button>
      </div>

      <!-- 파일 미리보기 (Step 3에서만 하단에 미리보기 표시) -->
      <div v-if="step === 3 && rawHeaders.length > 0">
        <div class="border-hr"></div>

        <div class="persistent-preview">

          <div class="persistent-preview-header" @click="previewCollapsed = !previewCollapsed">
            <span class="persistent-preview-label">{{ fileName }} · {{ rawData.length }}행</span>
            <span class="persistent-preview-toggle">{{ previewCollapsed ? '펼치기 ▾' : '접기 ▴' }}</span>
          </div>
          <div v-if="!previewCollapsed" class="preview-table-wrap">
            <table class="preview-table">
              <thead>
              <tr>
                <th v-for="h in rawHeaders" :key="h">{{ h }}</th>
              </tr>
              </thead>
              <tbody>
              <tr v-for="(row, i) in previewRows" :key="i">
                <td v-for="(cell, j) in row" :key="j">{{ cell }}</td>
              </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>


    </div>

    <!-- 하단 네비게이션 -->
    <div v-if="!importResult" class="wizard-footer">
      <button class="btn-prev" :disabled="step === 1" @click="goPrev">
        <ArrowLeft :size="15"/>
        이전
      </button>
      <button v-if="step < 6" class="btn-next" :disabled="!canGoNext" @click="goNext">
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

/* 툴바 */
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

/* 본문 */
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

.required {
  color: #f87171;
  margin-left: 2px;
}

.error-text {
  font-size: 14px;
  color: #f87171;
  margin: 12px 0 24px;
}

.col-map-error {
  font-size: 13px;
  color: #f87171;
  margin: 8px 0 0;
}

.empty-hint {
  font-size: 15px;
  color: var(--clr-text-subtle);
  margin: 0;
}

/* 단계 하단 미리보기 */
.persistent-preview {
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
  margin-top: 32px;
  margin-bottom: 8px;
}

.persistent-preview .preview-table-wrap {
  border: none;
  border-radius: 0;
}

.persistent-preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 14px;
  background: #0d1220;
  cursor: pointer;
  user-select: none;
}

.persistent-preview-label {
  font-size: 14px;
  color: var(--clr-text-subtle);
}

.persistent-preview-toggle {
  font-size: 13px;
  color: var(--clr-text-hint);
}

/* Step 1: 드롭존 */
.drop-zone {
  border: 2px dashed #1a2035;
  border-radius: 12px;
  padding: 48px 24px;
  text-align: center;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.drop-zone:hover,
.drop-zone--dragging {
  border-color: rgba(59, 91, 219, 0.5);
  background-color: rgba(59, 91, 219, 0.04);
}

.drop-zone--loaded {
  border-color: rgba(52, 211, 153, 0.4);
  border-style: solid;
}

.drop-icon {
  color: var(--clr-text-hint);
}

.drop-main {
  font-size: 16px;
  color: var(--clr-text-subtle);
  margin: 0;
}

.drop-main--loaded {
  color: #c8d8f0;
  font-weight: 600;
}

.drop-hint {
  font-size: 13px;
  color: var(--clr-text-hint);
  margin: 0;
}

/* 미리보기 테이블 */
.preview-block {
}

.preview-label {
  font-size: 14px;
  color: var(--clr-text-subtle);
  margin: 0 0 10px;
}

.preview-table-wrap {
  overflow-x: auto;
  border: 1px solid #1a2035;
  border-radius: 8px;
}

.preview-table {
  border-collapse: collapse;
  min-width: 100%;
  font-size: 14px;
}

.preview-table th {
  padding: 8px 12px;
  background: #0d1220;
  color: var(--clr-text-subtle);
  font-weight: 600;
  text-align: left;
  white-space: nowrap;
  border-bottom: 1px solid #1a2035;
}

.preview-table td {
  padding: 7px 12px;
  color: #c8d8f0;
  border-bottom: 1px solid rgba(26, 32, 53, 0.7);
  white-space: nowrap;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 예시 파일 다운로드 */
.sample-section {
}

.border-hr {
  margin-bottom: 32px;
  padding-bottom: 32px;
  border-bottom: 1px solid #1a2035;
}

.sample-type-card {
  cursor: default;
}

.sample-type-card:hover {
  border-color: #1a2035;
  background-color: transparent;
}

.btn-sample-dl {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 14px;
  padding: 7px 14px;
  background: rgba(59, 91, 219, 0.08);
  border: 1px solid rgba(59, 91, 219, 0.25);
  border-radius: 7px;
  color: #7ba8f0;
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.btn-sample-dl:hover {
  background: rgba(59, 91, 219, 0.18);
  color: #93c5fd;
}

/* Step 2: 타입 카드 */
.type-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(max(300px, calc(50% - 8px)), 1fr));
  gap: 16px;
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
  border-radius: 6px;
  padding: 2px 8px;
  color: #7ba8f0;
  background: rgba(59, 91, 219, 0.15);
  border: 1px solid rgba(59, 91, 219, 0.3);
}

.type-badge--a {
  color: #f87171;
  background: rgba(248, 113, 113, 0.12);
  border-color: rgba(248, 113, 113, 0.35);
}

.type-badge--b {
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.15);
  border-color: rgba(251, 191, 36, 0.4);
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

/* Step 3: 열 매핑 */
.id-mode-section {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 24px;
}

.id-mode-label {
  font-size: 15px;
  color: #c8d8f0;
  white-space: nowrap;
}

.id-mode-buttons {
  display: flex;
  gap: 0;
  border: 1px solid #1a2035;
  border-radius: 8px;
  overflow: hidden;
}

.id-mode-btn {
  padding: 7px 16px;
  background: none;
  border: none;
  color: var(--clr-text-subtle);
  font-size: 14px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.id-mode-btn + .id-mode-btn {
  border-left: 1px solid #1a2035;
}

.id-mode-btn--active {
  background: rgba(59, 91, 219, 0.12);
  color: #7ba8f0;
}

.sid-preview {
  margin-top: 4px;
  padding: 14px 16px;
  background: #0d1220;
  border: 1px solid #1a2035;
  border-radius: 8px;
}

.sid-preview-label {
  font-size: 13px;
  color: var(--clr-text-hint);
  margin: 0 0 10px;
}

.sid-preview-table {
  width: auto;
}

.sid-row-error td {
  color: #f87171;
}

.col-map-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.col-map-row {
  display: grid;
  grid-template-columns: 160px 1fr;
  align-items: center;
  gap: 16px;
}

.col-map-label {
  font-size: 15px;
  color: #c8d8f0;
}

.col-map-select {
  padding: 8px 12px;
  background: #0d1220;
  border: 1px solid #1a2035;
  border-radius: 8px;
  color: #e2e8f0;
  font-size: 14px;
  outline: none;
  cursor: pointer;
}

.col-map-select:focus {
  border-color: rgba(59, 91, 219, 0.5);
}

.activity-cols-preview {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
  min-height: 36px;
}

.activity-cols-hint {
  font-size: 13px;
  color: var(--clr-text-hint);
}

.activity-col-tag {
  font-size: 13px;
  color: #7ba8f0;
  background: rgba(59, 91, 219, 0.1);
  border: 1px solid rgba(59, 91, 219, 0.25);
  border-radius: 6px;
  padding: 3px 8px;
}

/* Step 4: 활동 매칭 */
.activity-match-list {
  display: flex;
  flex-direction: column;
  gap: 0;
  border: 1px solid #1a2035;
  border-radius: 10px;
  overflow: hidden;
}

.activity-match-header {
  display: grid;
  grid-template-columns: 1fr 32px 1fr;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  background: #0d1220;
  font-size: 13px;
  font-weight: 600;
  color: var(--clr-text-hint);
  border-bottom: 1px solid #1a2035;
}

.activity-match-row {
  display: grid;
  grid-template-columns: 1fr 32px 1fr;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  border-bottom: 1px solid rgba(26, 32, 53, 0.7);
}

.activity-match-row:last-child {
  border-bottom: none;
}

.import-act-name {
  font-size: 15px;
  color: #c8d8f0;
}

.match-arrow {
  font-size: 16px;
  color: var(--clr-text-hint);
  text-align: center;
}

.activity-match-select {
  padding: 7px 10px;
  background: #0d1220;
  border: 1px solid #1a2035;
  border-radius: 8px;
  color: #e2e8f0;
  font-size: 14px;
  outline: none;
  cursor: pointer;
  width: 100%;
}

.activity-match-select:focus {
  border-color: rgba(59, 91, 219, 0.5);
}

/* Step 5: 요약 & 결과 */
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

.btn-import {
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

.btn-import:hover:not(:disabled) {
  background: rgba(59, 91, 219, 0.25);
  color: #93c5fd;
}

.btn-import:disabled {
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

/* Step 5: Diff 미리보기 */
.diff-loading {
  padding: 48px 0;
  text-align: center;
  color: var(--clr-text-subtle);
  font-size: 15px;
}

.error-hint {
  font-size: 12px;
  color: var(--clr-text-hint);
}

.diff-stats-bar {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}

.diff-stat {
  font-size: 13px;
  font-weight: 600;
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid;
}

.diff-stat--changed {
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.1);
  border-color: rgba(251, 191, 36, 0.3);
}

.diff-stat--new {
  color: #34d399;
  background: rgba(52, 211, 153, 0.1);
  border-color: rgba(52, 211, 153, 0.3);
}

.diff-stat--same {
  color: var(--clr-text-hint);
  background: rgba(255, 255, 255, 0.03);
  border-color: #1a2035;
}

.diff-empty {
  padding: 40px 0;
  text-align: center;
}

.diff-empty-msg {
  font-size: 16px;
  color: #e2e8f0;
  margin: 0 0 8px;
}

.diff-empty-sub {
  font-size: 16px;
  color: var(--clr-text-subtle);
  margin: 4px 0 0;
}

.diff-controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
  flex-wrap: wrap;
  gap: 10px;
}

.diff-select-all {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #c8d8f0;
  cursor: pointer;
  user-select: none;
}

.diff-select-all input[type='checkbox'] {
  width: 15px;
  height: 15px;
  cursor: pointer;
  accent-color: #3b5bdb;
}

.diff-count {
  color: var(--clr-text-hint);
  font-size: 13px;
}

.diff-mode-buttons {
  display: flex;
  border: 1px solid #1a2035;
  border-radius: 8px;
  overflow: hidden;
}

.diff-mode-btn {
  padding: 6px 14px;
  background: none;
  border: none;
  color: var(--clr-text-subtle);
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.diff-mode-btn + .diff-mode-btn {
  border-left: 1px solid #1a2035;
}

.diff-mode-btn--active {
  background: rgba(59, 91, 219, 0.12);
  color: #7ba8f0;
}

.diff-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.diff-item {
  border: 1px solid rgba(251, 191, 36, 0.3);
  border-radius: 10px;
  overflow: hidden;
  transition: opacity 0.2s, border-color 0.2s;
}

.diff-item--unchecked {
  opacity: 0.45;
  border-color: #1a2035;
}

.diff-item-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: rgba(251, 191, 36, 0.05);
  border-bottom: 1px solid rgba(251, 191, 36, 0.15);
}

.diff-item--unchecked .diff-item-header {
  background: #0d1220;
  border-bottom-color: #1a2035;
}

.diff-checkbox {
  width: 15px;
  height: 15px;
  cursor: pointer;
  accent-color: #3b5bdb;
  flex-shrink: 0;
}

.diff-student-label {
  font-size: 14px;
  color: #c8d8f0;
  font-weight: 600;
}

.diff-separator {
  color: var(--clr-text-hint);
  font-size: 13px;
}

.diff-activity-label {
  font-size: 13px;
  color: #fbbf24;
}

.diff-boxes {
  display: grid;
  grid-template-columns: 1fr 1fr;
}

.diff-box {
  padding: 12px 14px;
}

.diff-box + .diff-box {
  border-left: 1px solid #1a2035;
}

.diff-box-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--clr-text-hint);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  margin-bottom: 6px;
}

.diff-box-content {
  font-size: 14px;
  color: #c8d8f0;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.diff-auto-note {
  font-size: 13px;
  margin-top: 14px;
}

.diff-auto-note--new {
  color: #34d399;
}

.diff-auto-note--same {
  color: var(--clr-text-hint);
}
</style>
