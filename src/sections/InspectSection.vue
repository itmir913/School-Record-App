<script setup>
import {computed, onMounted, ref, watch} from 'vue'
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
import {useFileStore} from '../stores/file'
import {performInspection} from '../services/synonymService'
import DiffView from '../components/DiffView.vue'
import WizardLayout from '../components/WizardLayout.vue'

const store = useSynonymStore()
const areaStore = useAreaStore()
const fileStore = useFileStore()

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
    await fileStore.writeBytesFile(filePath, data)
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
  <div class="flex flex-col h-full overflow-hidden box-border">

    <!-- 헤더 -->
    <div class="flex items-center justify-between px-10 pt-9 pb-6 border-b border-line shrink-0">
      <div class="flex flex-col">
        <h2 class="text-[22px] font-bold text-ink m-0 mb-1.5">유의어 점검(Inspect)</h2>
        <p class="text-base text-ink-3 m-0">중·고등학교 학교생활기록부 기재요령에 근거하여 유의어 및 금지어를 점검합니다.</p>
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

      <!-- ─── Step 1: 유의어 그룹 관리 ─────────────────────── -->
      <div v-if="step === 1">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 1. 유의어 관리</h3>
        <p class="text-base text-ink-5 m-0 mb-6">학교생활기록부 점검에 사용할 유의어 그룹과 검색 대상 단어를 관리합니다.</p>

        <!-- 로딩 -->
        <div v-if="store.loading" class="flex items-center gap-2.5 py-8 text-ink-5 text-sm">
          <Loader2 :size="22" class="animate-spin"/>
          불러오는 중...
        </div>

        <!-- 에러 -->
        <div v-else-if="store.error" class="msg-error">{{ store.error }}</div>

        <template v-else>

          <!-- 그룹 삭제 에러 -->
          <div v-if="removeGroupError" class="msg-error mb-3">{{ removeGroupError }}</div>

          <!-- 그룹 카드 목록 -->
          <div class="flex flex-col gap-4 mb-6">
            <div v-for="group in store.groups" :key="group.id"
                 class="bg-surface border border-line rounded-[10px] py-4 px-5 flex flex-col gap-3">

              <!-- 그룹 헤더 -->
              <div class="flex items-center gap-2.5">
                <span class="text-base font-semibold text-ink flex-1">{{ group.name }}</span>
                <span class="text-xs text-ink-5 bg-line py-[2px] px-2 rounded-xl">{{ group.items.length }}개</span>
                <button
                    class="flex items-center justify-center w-7 h-7 border-none rounded-[6px] cursor-pointer bg-transparent text-red transition-colors hover:bg-red/[0.15]"
                    @click="removeGroup(group.id)" title="그룹 삭제"
                >
                  <Trash2 :size="14"/>
                </button>
              </div>

              <!-- 단어 chips -->
              <div class="flex flex-wrap gap-1.5 min-h-7">
                <span v-for="item in group.items" :key="item.id"
                      class="inline-flex items-center gap-1 bg-line border border-line-2 rounded-[20px] py-[3px] px-2.5 text-sm text-ink-2">
                  {{ item.word }}
                  <button
                      class="flex items-center bg-transparent border-none cursor-pointer text-ink-4 p-0 ml-0.5 transition-colors hover:text-red"
                      @click="store.deleteWord(item.id)"
                  >
                    <X :size="11"/>
                  </button>
                </span>
                <span v-if="group.items.length === 0" class="text-xs text-ink-4 italic">단어 없음</span>
              </div>

              <!-- 단어 추가 입력 -->
              <div class="flex gap-2 items-center">
                <input
                    class="flex-1 bg-line border border-line-2 rounded-[6px] py-1.5 px-2.5 text-sm text-ink outline-none transition-colors focus:border-blue"
                    placeholder="단어 추가..."
                    :value="getWordInput(group.id)"
                    @input="setWordInput(group.id, $event.target.value)"
                    @keydown.enter="submitWord(group.id)"
                />
                <button
                    class="inline-flex items-center gap-1 py-1.5 px-3 rounded-[6px] border-none text-sm font-medium cursor-pointer transition-colors bg-blue text-white hover:bg-blue-2"
                    @click="submitWord(group.id)"
                >
                  <Plus :size="13"/>
                  추가
                </button>
              </div>

              <!-- 유의어 일괄 등록 -->
              <details class="border-t border-line pt-2.5">
                <summary class="text-xs text-ink-5 cursor-pointer select-none list-none [&::-webkit-details-marker]:hidden">
                  유의어 일괄 등록
                </summary>
                <div class="flex flex-col gap-2 mt-2.5">
                  <textarea
                      class="w-full min-h-[80px] bg-line border border-line-2 rounded-[6px] py-2 px-2.5 text-sm text-ink resize-y outline-none box-border transition-colors focus:border-blue"
                      placeholder="줄바꿈 또는 쉼표로 구분하여 여러 단어를 입력하세요"
                      :value="getCsvInput(group.id)"
                      @input="setCsvInput(group.id, $event.target.value)"
                  />
                  <button
                      class="inline-flex items-center gap-1 py-1.5 px-3 rounded-[6px] border-none text-sm font-medium cursor-pointer transition-colors bg-blue text-white enabled:hover:bg-blue-2 disabled:opacity-50 disabled:cursor-not-allowed"
                      :disabled="csvUploading[group.id]"
                      @click="submitCsv(group.id)"
                  >
                    <Loader2 v-if="csvUploading[group.id]" :size="13" class="animate-spin"/>
                    <span v-else>업로드</span>
                  </button>
                </div>
              </details>

            </div>
          </div>

          <!-- 그룹 추가 -->
          <div class="mt-1">
            <template v-if="showAddGroup">
              <div class="flex gap-2 items-center">
                <input
                    v-model="newGroupName"
                    class="flex-1 bg-line border border-line-2 rounded-[6px] py-2 px-3 text-sm text-ink outline-none transition-colors focus:border-blue"
                    placeholder="새 그룹 이름"
                    @keydown.enter="submitNewGroup"
                    @keydown.esc="showAddGroup = false; addGroupError = ''"
                    autofocus
                />
                <button
                    class="inline-flex items-center gap-1 py-1.5 px-3 rounded-[6px] border-none text-sm font-medium cursor-pointer transition-colors bg-blue text-white hover:bg-blue-2"
                    @click="submitNewGroup"
                >
                  <Plus :size="13"/>
                  생성
                </button>
                <button
                    class="py-2 px-3.5 border border-line-2 rounded-[6px] bg-transparent text-ink-5 text-sm cursor-pointer transition-[border-color,color] hover:border-ink-5 hover:text-ink-2"
                    @click="showAddGroup = false; addGroupError = ''"
                >취소</button>
              </div>
              <p v-if="addGroupError" class="text-xs text-red m-0 mt-1.5">{{ addGroupError }}</p>
            </template>
            <button v-else
                    class="inline-flex items-center gap-1.5 py-2 px-4 bg-transparent border border-dashed border-line-2 rounded-lg text-ink-5 text-sm cursor-pointer transition-[border-color,color] hover:border-blue hover:text-violet"
                    @click="showAddGroup = true"
            >
              <Plus :size="15"/>
              그룹 추가
            </button>
          </div>

        </template>
      </div>

      <!-- ─── Step 2: 그룹 선택 ─────────────────────────────── -->
      <div v-if="step === 2">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 2. 점검할 그룹 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">검색에 사용할 유의어 그룹을 하나 이상 선택하세요.</p>

        <div class="flex flex-col gap-2 mb-5">
          <label
              v-for="group in store.groups"
              :key="group.id"
              class="flex items-center gap-3.5 py-3.5 px-[18px] bg-surface border border-line rounded-[10px] cursor-pointer transition-[border-color,background-color]"
              :class="isSelected(group.id) ? 'border-blue bg-blue/[0.08]' : 'hover:border-line-2 hover:bg-raised'"
          >
            <input
                type="checkbox"
                :checked="isSelected(group.id)"
                @change="toggleGroup(group.id)"
                class="sr-only"
            />
            <div
                class="flex items-center justify-center w-5 h-5 rounded-[5px] border-[1.5px] shrink-0 transition-[border-color,background-color]"
                :class="isSelected(group.id) ? 'border-blue bg-blue text-white' : 'border-line-2 bg-transparent text-green'"
            >
              <Check v-if="isSelected(group.id)" :size="13"/>
            </div>
            <div class="flex-1 flex flex-col gap-0.5">
              <span class="text-sm font-semibold text-ink">{{ group.name }}</span>
              <span class="text-xs text-ink-5">{{ group.items.length }}개 단어</span>
            </div>
            <ChevronRight :size="16" class="text-ink-5"/>
          </label>
        </div>

        <div class="text-sm text-ink-5 mb-5">
          선택된 그룹: <strong class="text-violet">{{ selectedGroupIds.length }}개</strong> &nbsp;|&nbsp;
          중복 제거 단어: <strong class="text-violet">{{ selectedWordCount }}개</strong>
        </div>
      </div>

      <!-- ─── Step 3: 점검 범위 선택 ──────────────────────────── -->
      <div v-if="step === 3">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 3. 점검 범위 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">유의어를 점검할 생기부 범위를 선택하세요.</p>

        <!-- 모드 카드 2개: 전체 영역 / 특정 영역 -->
        <div class="grid grid-cols-2 gap-3 mb-6">
          <div
              class="relative border-2 rounded-[10px] py-[18px] px-5 cursor-pointer flex flex-col gap-1.5 transition-[border-color,background-color] duration-200"
              :class="scopeMode === 'all' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="scopeMode = 'all'"
          >
            <Check v-if="scopeMode === 'all'" :size="14" class="absolute top-2.5 right-3 text-blue-2"/>
            <span class="text-base font-semibold text-ink">전체 영역</span>
            <span class="text-sm text-ink-5">모든 생기부 기록을 점검합니다</span>
          </div>
          <div
              class="relative border-2 rounded-[10px] py-[18px] px-5 cursor-pointer flex flex-col gap-1.5 transition-[border-color,background-color] duration-200"
              :class="scopeMode === 'specific' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="scopeMode = 'specific'"
          >
            <Check v-if="scopeMode === 'specific'" :size="14" class="absolute top-2.5 right-3 text-blue-2"/>
            <span class="text-base font-semibold text-ink">특정 영역</span>
            <span class="text-sm text-ink-5">점검할 영역을 직접 선택합니다</span>
          </div>
        </div>

        <!-- 개별 영역 카드 그리드 -->
        <div
            class="mb-5 transition-opacity"
            :class="{ 'opacity-35 pointer-events-none': scopeMode !== 'specific' }"
        >
          <p class="text-sm text-ink-5 m-0 mb-3">점검할 영역 선택</p>
          <p v-if="areaStore.areas.length === 0" class="text-sm text-ink-4">등록된 영역이 없습니다.</p>
          <div v-else class="grid gap-3 grid-cards-200">
            <div
                v-for="area in areaStore.areas"
                :key="area.id"
                class="relative border-2 rounded-[10px] py-4 px-5 cursor-pointer transition-[border-color,background-color] duration-200 flex flex-col gap-1.5"
                :class="isAreaSelected(area.id) ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
                @click="toggleArea(area.id)"
            >
              <Check v-if="isAreaSelected(area.id)" :size="14" class="absolute top-2.5 right-3 text-blue-2"/>
              <span class="text-base font-semibold text-ink">{{ area.name }}</span>
              <span class="text-sm text-ink-5">활동 {{ area.activities.length }}개</span>
            </div>
          </div>
          <p v-if="scopeMode === 'specific' && selectedAreaIds.length > 0" class="text-sm text-ink-5 mt-3">
            {{ selectedAreaIds.length }}개 영역 선택됨
          </p>
        </div>

        <!-- 검색 에러 -->
        <div v-if="searchError" class="msg-error mb-3">{{ searchError }}</div>

        <!-- 검색 시작 버튼 -->
        <button
            class="inline-flex items-center gap-2 py-3 px-7 bg-blue border-none rounded-lg text-white text-base font-semibold cursor-pointer transition-colors enabled:hover:bg-blue-2 disabled:opacity-45 disabled:cursor-not-allowed"
            :disabled="searching || (scopeMode === 'specific' && selectedAreaIds.length === 0)"
            @click="startSearch"
        >
          <Loader2 v-if="searching" :size="16" class="animate-spin"/>
          <ScanSearch v-else :size="16"/>
          {{ searching ? '검색 중...' : '검색 시작' }}
        </button>
      </div>

      <!-- ─── Step 4: 결과 보고 ──────────────────────────────── -->
      <div v-if="step === 4">

        <!-- 점검 결과 헤더 -->
        <div class="flex items-start justify-between gap-5 mb-6">
          <div>
            <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 4. 점검 결과</h3>
            <p v-if="inspectResults.length > 0" class="text-base text-ink-5 m-0">
              총 <strong>{{ inspectResults.length }}건</strong>의 기록에서 유의어가 탐지되었습니다.
            </p>
            <p v-else class="text-base text-ink-5 m-0">
              선택한 유의어 그룹에 해당하는 기록이 없습니다.
            </p>
          </div>
          <button v-if="inspectResults.length > 0"
                  class="inline-flex items-center gap-[7px] py-[9px] px-[18px] bg-green/25 border border-green/40 rounded-lg text-green text-sm font-semibold cursor-pointer whitespace-nowrap shrink-0 transition-colors enabled:hover:bg-green/35 disabled:opacity-50 disabled:cursor-not-allowed"
                  :disabled="exporting"
                  @click="exportToExcel"
          >
            <Loader2 v-if="exporting" :size="15" class="animate-spin"/>
            <FileDown v-else :size="15"/>
            {{ exporting ? '저장 중...' : 'Excel 내보내기' }}
          </button>
        </div>

        <div v-if="inspectResults.length === 0" class="flex flex-col items-center justify-center py-16 gap-3">
          <ScanSearch :size="40" class="text-ink-5"/>
          <p class="text-lg font-semibold text-ink-5 m-0">탐지된 유의어가 없습니다</p>
          <p class="text-base text-ink-4 m-0">유의어가 발견되지 않았습니다.</p>
        </div>

        <div v-else class="overflow-x-auto border border-line rounded-[10px]">
          <table class="w-full border-collapse text-sm [&_tr:hover_td]:bg-blue/[0.04] [&_tr:last-child_td]:border-b-0">
            <thead class="bg-surface">
            <tr>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">학년</th>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">반</th>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">번호</th>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">이름</th>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">영역</th>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">활동</th>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">원본 내용</th>
              <th class="py-2.5 px-3.5 text-left font-semibold text-ink-5 border-b border-line whitespace-nowrap">발견된 유의어</th>
            </tr>
            </thead>
            <tbody>
            <tr v-for="{ record, detectedWords } in inspectResults" :key="record.id">
              <td class="py-2.5 px-3.5 align-top border-b border-raised text-ink-2 whitespace-nowrap text-xs text-ink-4">{{ record.grade || '' }}</td>
              <td class="py-2.5 px-3.5 align-top border-b border-raised text-ink-2 whitespace-nowrap text-xs text-ink-4">{{ record.class_num || '' }}</td>
              <td class="py-2.5 px-3.5 align-top border-b border-raised text-ink-2 whitespace-nowrap text-xs text-ink-4">{{ record.number || '—' }}</td>
              <td class="py-2.5 px-3.5 align-top border-b border-raised text-ink-2 whitespace-nowrap text-xs text-ink-4">{{ record.student_name || '—' }}</td>
              <td class="py-2.5 px-3.5 align-top border-b border-raised whitespace-nowrap text-ink-5 text-xs">{{ record.area_name || '—' }}</td>
              <td class="py-2.5 px-3.5 align-top border-b border-raised text-ink-2 whitespace-nowrap font-medium">{{ record.activity_name }}</td>
              <td class="cell-content py-2.5 px-3.5 align-top border-b border-raised text-ink-2 leading-relaxed break-all max-w-[400px]">
                <DiffView
                    :before="buildBeforeText(record.content, detectedWords)"
                    :after="record.content"
                />
              </td>
              <td class="py-2.5 px-3.5 align-top border-b border-raised text-ink-2 whitespace-nowrap">
                <span
                    v-for="word in detectedWords"
                    :key="word"
                    class="inline-block my-[2px] mr-[3px] py-[2px] px-2 bg-violet/[0.12] border border-violet/25 rounded-xl text-xs text-violet"
                >{{ word }}</span>
              </td>
            </tr>
            </tbody>
          </table>
        </div>

      </div>

      <!-- 내보내기 완료 화면 -->
      <div v-if="step === 5 && exportResult" class="flex flex-col items-center gap-4 py-16">
        <div class="text-[48px] text-green">✓</div>
        <p class="text-[22px] font-bold text-ink m-0">내보내기 완료</p>
        <div class="flex gap-8">
          <div class="flex flex-col items-center gap-1">
            <span class="text-[36px] font-bold text-blue-2">{{ exportResult.rowCount }}</span>
            <span class="text-sm text-ink-5">행 저장됨</span>
          </div>
        </div>
        <p class="text-sm text-ink-5 m-0">{{ exportResult.fileName }}</p>
        <div class="flex gap-2.5 mt-2">
          <button
              class="py-[9px] px-6 bg-blue/[0.12] border border-blue/35 rounded-lg text-blue-2 text-base cursor-pointer transition-colors hover:bg-blue/[0.22] hover:text-ink-2"
              @click="revealItemInDir(exportResult.filePath)"
          >파일 확인</button>
          <button
              class="py-[9px] px-6 bg-transparent border border-line rounded-lg text-ink-5 text-base cursor-pointer transition-colors hover:bg-line hover:text-ink-3"
              @click="resetWizard"
          >처음으로 돌아가기</button>
        </div>
      </div>

    </WizardLayout>

  </div>
</template>

<style scoped>
/* DiffView 내부 유의어 하이라이트 — :deep() 필수 */
.cell-content :deep(.diff-added) {
  background-color: color-mix(in srgb, var(--c-amber) 30%, transparent);
  color: var(--c-amber);
}

.cell-content :deep(.diff-removed) {
  display: none;
}
</style>
