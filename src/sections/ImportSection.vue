<script setup>
import {computed, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {ArrowLeft, ArrowRight, FileSpreadsheet} from 'lucide-vue-next'
import * as XLSX from 'xlsx'

const COL_ALIASES = {
  grade: ['학년', 'grade'],
  classNum: ['반', 'class', '학급', '반번호', 'classnum', 'class_num'],
  number: ['번호', 'number', 'num', '번', '출석번호'],
  name: ['이름', 'name', '성명', '학생명', '학생이름'],
  activityName: ['활동명', '활동', '분류', 'activity', 'activity_name', 'activityname'],
  activityContent: ['활동내용', '내용', 'content', '기록', '활동기록'],
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

const studentIdPreviewRows = computed(() => {
  const col = colMap.value.studentId
  if (col === null) return []
  return rawData.value.slice(0, 5).map(row => {
    const raw = row[col]
    const parsed = parseStudentId(raw)
    return parsed ? {raw, ...parsed, error: false} : {raw, grade: '?', classNum: '?', number: '?', error: true}
  })
})

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

  const reader = new FileReader()
  reader.onerror = () => {
    parseError.value = '파일을 읽을 수 없습니다. 파일 권한을 확인해주세요.'
  }
  reader.onload = (ev) => {
    try {
      const data = new Uint8Array(ev.target.result)
      const wb = XLSX.read(data, {type: 'array'})
      const ws = wb.Sheets[wb.SheetNames[0]]
      const rows = XLSX.utils.sheet_to_json(ws, {header: 1, defval: ''})
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
  if (step.value === 3) {
    await loadDbActivities()
    initActivityMatchMap()
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
    const m = colMap.value

    function resolveIdentity(row) {
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

    if (fileType.value === 'A') {
      for (const row of rawData.value) {
        const identity = resolveIdentity(row)
        if (!identity) continue
        const actName = String(row[m.activityName] ?? '').trim()
        const content = String(row[m.activityContent] ?? '').trim()
        if (!actName || !content) continue
        const activity_id = finalMap[actName]
        if (!activity_id) continue
        records.push({
          ...identity, activity_id, content,
          name: m.name !== null ? (String(row[m.name] ?? '').trim() || null) : null,
        })
      }
    } else {
      for (const row of rawData.value) {
        const identity = resolveIdentity(row)
        if (!identity) continue
        for (const {name: actName, index} of activityColIndices.value) {
          const content = String(row[index] ?? '').trim()
          if (!content) continue
          const activity_id = finalMap[actName]
          if (!activity_id) continue
          records.push({
            ...identity, activity_id, content,
            name: m.name !== null ? (String(row[m.name] ?? '').trim() || null) : null,
          })
        }
      }
    }

    if (records.length === 0) {
      importError.value = '가져올 데이터가 없습니다.'
      return
    }

    importResult.value = await invoke('bulk_import_records', {records})
  } catch (e) {
    importError.value = '가져오기 실패: ' + String(e)
  } finally {
    importing.value = false
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
}
</script>

<template>
  <div class="section">

    <!-- 툴바 -->
    <div class="toolbar">
      <h2 class="section-title">데이터 가져오기</h2>
      <div class="step-indicator">
        <span
            v-for="n in 5"
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
        <h3 class="step-title">가져올 파일 선택</h3>
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
        <h3 class="step-title">파일 양식 선택</h3>
        <p class="step-desc">파일의 구조에 맞는 양식을 선택하세요.</p>

        <div class="type-cards">
          <div
              class="type-card"
              :class="{ 'type-card--selected': fileType === 'A' }"
              @click="fileType = 'A'"
          >
            <div class="type-card-top">
              <span class="type-badge">A 타입</span>
              <span class="type-name">행별 활동 형식</span>
            </div>
            <p class="type-desc">한 행에 학생 1명의 활동 1개가 기재됩니다.<br>여러 활동을 한 파일에서 동시에 가져올 수 있습니다.</p>
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
                  <td>환경부원으로...</td>
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
              <span class="type-badge">B 타입</span>
              <span class="type-name">열별 활동 형식</span>
            </div>
            <p class="type-desc">활동이 열(헤더)로 나뉜 형식입니다.<br>한 학생의 여러 활동이 한 행에 있습니다.</p>
            <div class="sample-table-wrap">
              <table class="sample-table">
                <thead>
                <tr>
                  <th>학년</th>
                  <th>반</th>
                  <th>번호</th>
                  <th>이름</th>
                  <th>학급자치회</th>
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
                  <td>환경부장으로...</td>
                  <td>독서 동아리에서...</td>
                </tr>
                <tr>
                  <td>3</td>
                  <td>1</td>
                  <td>3</td>
                  <td>박영희</td>
                  <td>부회장으로...</td>
                  <td>컴퓨터 동아리에서...</td>
                </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <!-- Step 3: 열 매핑 -->
      <div v-else-if="step === 3" class="step-content">
        <h3 class="step-title">열 매핑</h3>
        <p class="step-desc">파일의 각 열을 해당 필드에 연결하세요. <span class="required">*</span> 는 필수입니다.</p>

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
              <p class="sid-preview-label">파싱 미리보기 (5자리: ABBCC · 6자리: ABBCCC)</p>
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
        <h3 class="step-title">활동 매칭</h3>
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

      <!-- Step 5: 실행 -->
      <div v-else-if="step === 5" class="step-content">
        <h3 class="step-title">가져오기 실행</h3>

        <div v-if="!importResult">
          <div class="summary-box">
            <div class="summary-row">
              <span class="summary-key">파일</span>
              <span class="summary-val">{{ fileName }}</span>
            </div>
            <div class="summary-row">
              <span class="summary-key">양식</span>
              <span class="summary-val">{{ fileType === 'A' ? 'A타입 — 행별 활동 형식' : 'B타입 — 열별 활동 형식' }}</span>
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

        <div v-else class="result-box">
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
      </div>

      <!-- 파일 미리보기 (Step 2 이후 하단 표시) -->
      <div v-if="step > 1 && step < 5 && rawHeaders.length > 0" class="persistent-preview">
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

    <!-- 하단 네비게이션 -->
    <div v-if="!importResult" class="wizard-footer">
      <button class="btn-prev" :disabled="step === 1" @click="goPrev">
        <ArrowLeft :size="15"/>
        이전
      </button>
      <button v-if="step < 5" class="btn-next" :disabled="!canGoNext" @click="goNext">
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
  margin: 12px 0 0;
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
  margin-top: 40px;
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
  padding: 40px 24px;
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
  margin-top: 24px;
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

/* Step 2: 타입 카드 */
.type-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
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
  color: #7ba8f0;
  background: rgba(59, 91, 219, 0.15);
  border: 1px solid rgba(59, 91, 219, 0.3);
  border-radius: 6px;
  padding: 2px 8px;
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
</style>
