<script setup>
import {computed, onMounted, ref, watch} from 'vue'
import {useReplaceRuleStore} from '../stores/replaceRule'
import {useAreaStore} from '../stores/area'
import DiffView from '../components/DiffView.vue'
import {
  Check,
  ChevronLeft,
  ChevronRight,
  Eye,
  Pencil,
  Play,
  Plus,
  SearchX,
  Trash2,
  TriangleAlert,
  X,
} from 'lucide-vue-next'
import WizardLayout from '../components/WizardLayout.vue'

const ruleStore = useReplaceRuleStore()
const areaStore = useAreaStore()

// ── 단계 ──────────────────────────────────────────────────────
const step = ref(1)

// ── 규칙 인라인 편집 ───────────────────────────────────────
const editingId = ref(null)
const editForm = ref({oldText: '', newText: '', priority: 0, isRegex: false})
const editError = ref('')
const operationError = ref('')
const isAdjusting = ref(false)

function startEdit(rule) {
  editingId.value = rule.id
  editForm.value = {oldText: rule.old_text, newText: rule.new_text, priority: rule.priority, isRegex: rule.is_regex}
}

function cancelEdit() {
  editingId.value = null
}

async function commitEdit(rule) {
  editError.value = ''
  try {
    await ruleStore.updateRule(
        rule.id,
        editForm.value.oldText,
        editForm.value.newText,
        rule.enabled,
        editForm.value.priority,
        editForm.value.isRegex,
    )
    editingId.value = null
  } catch (e) {
    editError.value = e?.toString() ?? '수정 실패'
  }
}

async function toggleEnabled(rule) {
  operationError.value = ''
  try {
    await ruleStore.updateRule(
        rule.id, rule.old_text, rule.new_text, !rule.enabled, rule.priority, rule.is_regex,
    )
  } catch (e) {
    operationError.value = e?.toString() ?? '수정 실패'
  }
}

async function adjustPriority(rule, delta) {
  if (isAdjusting.value) return
  isAdjusting.value = true
  const newPriority = rule.priority + delta
  operationError.value = ''
  try {
    await ruleStore.updateRule(
        rule.id, rule.old_text, rule.new_text, rule.enabled, newPriority, rule.is_regex,
    )
  } catch (e) {
    operationError.value = e?.toString() ?? '수정 실패'
  } finally {
    isAdjusting.value = false
  }
}

async function deleteRule(id) {
  operationError.value = ''
  try {
    await ruleStore.deleteRule(id)
  } catch (e) {
    operationError.value = e?.toString() ?? '삭제 실패'
  }
}

// ── 규칙 추가 폼 ───────────────────────────────────────────
const showAddForm = ref(false)
const newRule = ref({oldText: '', newText: '', priority: 0, isRegex: false})
const addError = ref('')

async function submitAdd() {
  addError.value = ''
  try {
    await ruleStore.createRule(newRule.value.oldText, newRule.value.newText, newRule.value.priority, newRule.value.isRegex)
    newRule.value = {oldText: '', newText: '', priority: 0, isRegex: false}
    showAddForm.value = false
  } catch (e) {
    addError.value = e?.toString() ?? '추가 실패'
  }
}

// ── Step 2: 범위 선택 ─────────────────────────────────────
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
  if (step.value === 2) return !(scopeMode.value === 'specific' && selectedAreaIds.value.length === 0)
  return true
})

function goPrev() {
  step.value--
}

function goNext() {
  step.value++
}

// ── 미리보기 ───────────────────────────────────────────────
const previewItems = ref([])
const previewError = ref('')
const isPreviewing = ref(false)
const hasRanPreview = ref(false)
const PREVIEW_LIMIT = 50
const showAllPreview = ref(false)

const visiblePreviewItems = computed(() =>
    showAllPreview.value ? previewItems.value : previewItems.value.slice(0, PREVIEW_LIMIT),
)

function scopeArgs() {
  return scopeMode.value === 'specific'
      ? ['areas', selectedAreaIds.value]
      : ['all', []]
}

async function runPreview() {
  previewError.value = ''
  applyError.value = ''
  previewItems.value = []
  applyResult.value = null
  hasRanPreview.value = false
  isPreviewing.value = true
  try {
    const [st, ids] = scopeArgs()
    previewItems.value = await ruleStore.previewReplace(st, ids)
    showAllPreview.value = false
  } catch (e) {
    previewError.value = e?.toString() ?? '미리보기 실패'
  } finally {
    isPreviewing.value = false
    hasRanPreview.value = true
  }
}

// ── 적용 ──────────────────────────────────────────────────
const applyResult = ref(null)
const applyError = ref('')
const isApplying = ref(false)

async function runApply() {
  applyError.value = ''
  previewError.value = ''
  isApplying.value = true
  try {
    const [st, ids] = scopeArgs()
    applyResult.value = await ruleStore.applyReplace(st, ids)
    previewItems.value = []
    step.value++
  } catch (e) {
    applyError.value = e?.toString() ?? '적용 실패'
  } finally {
    isApplying.value = false
  }
}

function resetWizard() {
  step.value = 1
  editingId.value = null
  editError.value = ''
  operationError.value = ''
  isAdjusting.value = false
  showAddForm.value = false
  newRule.value = {oldText: '', newText: '', priority: 0, isRegex: false}
  addError.value = ''
  scopeMode.value = 'all'
  selectedAreaIds.value = []
  previewItems.value = []
  previewError.value = ''
  isPreviewing.value = false
  hasRanPreview.value = false
  showAllPreview.value = false
  applyResult.value = null
  applyError.value = ''
  isApplying.value = false
}

// ── 마운트 ─────────────────────────────────────────────────
onMounted(async () => {
  await Promise.all([ruleStore.fetchRules(), areaStore.fetchAreas()])
})
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden box-border">

    <!-- 헤더 -->
    <div class="flex items-center justify-between px-10 pt-9 pb-6 border-b border-line shrink-0">
      <div class="flex flex-col">
        <h2 class="text-[22px] font-bold text-ink m-0 mb-1.5">텍스트 치환(Replace)</h2>
        <p class="text-base text-ink-3 m-0">학교생활기록부 문장의 특수문자, 텍스트 등을 일괄 교체합니다.</p>
      </div>
    </div>

    <WizardLayout
        :stepCount="3"
        :currentStep="step"
        :canGoNext="canGoNext"
        :isNavigating="false"
        :showFooter="!applyResult"
        @prev="goPrev"
        @next="goNext"
    >

      <!-- ─── Step 1: 규칙 관리 ─────────────────────────────── -->
      <div v-if="step === 1">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 1. 텍스트 치환 규칙</h3>
        <p class="text-base text-ink-5 m-0 mb-6">찾아 바꿀 텍스트 규칙을 관리합니다. <u>우선순위 숫자가 작은 규칙부터 먼저 실행</u>됩니다.</p>

        <div class="bg-surface border border-line rounded-[10px] p-4">
          <div class="flex items-center justify-between mb-3">
            <span class="text-lg font-semibold text-ink-4 uppercase tracking-[0.05em]">치환 규칙 목록</span>
            <button
                class="inline-flex items-center gap-[5px] py-1.5 px-3 rounded-[6px] border-none text-sm cursor-pointer transition-colors bg-blue/20 text-blue-2 hover:bg-blue/[0.35]"
                @click="showAddForm = !showAddForm"
            >
              <Plus :size="14"/>
              규칙 추가
            </button>
          </div>

          <!-- 추가 폼 -->
          <div v-if="showAddForm" class="flex items-center gap-2 flex-wrap p-3 bg-raised rounded-lg mb-2.5">
            <input
                v-model="newRule.oldText"
                class="bg-raised border border-line-2 rounded-[6px] text-ink-2 text-base py-1.5 px-2.5 outline-none flex-1 min-w-[80px] focus:border-blue"
                placeholder="찾을 텍스트"
                @keydown.enter="submitAdd"
            />
            <span class="text-ink-5 shrink-0 text-base">→</span>
            <input
                v-model="newRule.newText"
                class="bg-raised border border-line-2 rounded-[6px] text-ink-2 text-base py-1.5 px-2.5 outline-none flex-1 min-w-[80px] focus:border-blue"
                placeholder="바꿀 텍스트"
                @keydown.enter="submitAdd"
            />
            <input
                v-model.number="newRule.priority"
                type="number"
                class="bg-raised border border-line-2 rounded-[6px] text-ink-2 text-base py-1.5 px-2.5 outline-none flex-[0_0_68px] min-w-[68px] focus:border-blue"
                placeholder="우선순위"
                @keydown.enter="submitAdd"
            />
            <label class="flex items-center gap-1 text-sm text-ink-5 whitespace-nowrap cursor-pointer shrink-0">
              <input type="checkbox" v-model="newRule.isRegex"/>
              정규식
            </label>
            <button
                class="inline-flex items-center justify-center w-7 h-7 rounded-[5px] border-none bg-transparent text-green cursor-pointer transition-[background-color,color] shrink-0 hover:bg-green/10"
                @click="submitAdd" title="추가"
            >
              <Check :size="14"/>
            </button>
            <button
                class="inline-flex items-center justify-center w-7 h-7 rounded-[5px] border-none bg-transparent text-red cursor-pointer transition-[background-color,color] shrink-0 hover:bg-red/10"
                @click="showAddForm = false; addError = ''" title="취소"
            >
              <X :size="14"/>
            </button>
            <p v-if="addError" class="text-red text-sm m-0 mt-1 w-full">{{ addError }}</p>
          </div>

          <p v-if="ruleStore.error" class="text-red text-sm m-0 mt-2">{{ ruleStore.error }}</p>
          <p v-if="operationError" class="text-red text-sm m-0 mt-2">{{ operationError }}</p>

          <!-- 규칙 테이블 -->
          <div v-if="ruleStore.rules.length === 0 && !ruleStore.loading" class="text-line-2 text-sm py-3.5 text-center">
            규칙이 없습니다. 추가 버튼으로 새 규칙을 만드세요.
          </div>

          <div v-else class="flex flex-col gap-0.5">
            <!-- 헤더 행 -->
            <div class="grid gap-2 items-center py-1 px-2.5 text-sm text-ink-5 uppercase tracking-[0.05em]"
                 style="grid-template-columns: 72px 1fr 36px 1fr 64px 76px">
              <span class="flex justify-center text-center">우선순위</span>
              <span>찾을 텍스트</span>
              <span class="flex justify-center text-center">→</span>
              <span>바꿀 텍스트</span>
              <span class="flex justify-center text-center">활성화</span>
              <span class="flex justify-center text-center">편집</span>
            </div>

            <div
                v-for="rule in ruleStore.rules"
                :key="rule.id"
                class="grid gap-2 items-center py-2 px-2.5 rounded-[6px] transition-colors hover:bg-raised"
                :class="{ 'opacity-45': !rule.enabled }"
                style="grid-template-columns: 72px 1fr 36px 1fr 64px 76px"
            >
              <!-- 인라인 편집 모드 -->
              <template v-if="editingId === rule.id">
                <div class="col-span-full flex items-center gap-2 flex-wrap">
                  <input v-model.number="editForm.priority" type="number"
                         class="bg-raised border border-line-2 rounded-[6px] text-ink-2 text-base py-1.5 px-2.5 outline-none flex-[0_0_68px] min-w-[68px] focus:border-blue"
                         title="우선순위"/>
                  <input v-model="editForm.oldText"
                         class="bg-raised border border-line-2 rounded-[6px] text-ink-2 text-base py-1.5 px-2.5 outline-none flex-1 min-w-[80px] focus:border-blue"
                         placeholder="찾을 텍스트"/>
                  <span class="text-ink-5 shrink-0 text-base">→</span>
                  <input v-model="editForm.newText"
                         class="bg-raised border border-line-2 rounded-[6px] text-ink-2 text-base py-1.5 px-2.5 outline-none flex-1 min-w-[80px] focus:border-blue"
                         placeholder="바꿀 텍스트"/>
                  <label class="flex items-center gap-1 text-sm text-ink-5 whitespace-nowrap cursor-pointer shrink-0">
                    <input type="checkbox" v-model="editForm.isRegex"/>
                    정규식
                  </label>
                  <button
                      class="inline-flex items-center justify-center w-7 h-7 rounded-[5px] border-none bg-transparent text-green cursor-pointer shrink-0 hover:bg-green/10"
                      @click="commitEdit(rule)" title="저장"
                  >
                    <Check :size="15"/>
                  </button>
                  <button
                      class="inline-flex items-center justify-center w-7 h-7 rounded-[5px] border-none bg-transparent text-red cursor-pointer shrink-0 hover:bg-red/10"
                      @click="cancelEdit" title="취소"
                  >
                    <X :size="15"/>
                  </button>
                </div>
                <p v-if="editError" class="text-red text-sm m-0 mt-1 w-full col-span-full">{{ editError }}</p>
              </template>

              <!-- 표시 모드 -->
              <template v-else>
                <!-- 우선순위 컨트롤 -->
                <div class="flex flex-row items-center justify-center gap-1">
                  <button
                      class="flex items-center justify-center w-5 h-5 rounded-[3px] border-none bg-transparent text-ink-5 cursor-pointer hover:bg-raised hover:text-ink-4"
                      :disabled="isAdjusting" @click="adjustPriority(rule, -1)" title="우선순위 높이기"
                  >
                    <ChevronLeft :size="14"/>
                  </button>
                  <span class="text-sm text-ink-5 min-w-[26px] text-center tabular-nums">{{ rule.priority }}</span>
                  <button
                      class="flex items-center justify-center w-5 h-5 rounded-[3px] border-none bg-transparent text-ink-5 cursor-pointer hover:bg-raised hover:text-ink-4"
                      :disabled="isAdjusting" @click="adjustPriority(rule, 1)" title="우선순위 낮추기"
                  >
                    <ChevronRight :size="14"/>
                  </button>
                </div>

                <!-- 찾을 텍스트 -->
                <div class="flex items-center gap-1.5 overflow-hidden justify-end">
                  <span v-if="rule.is_regex"
                        class="inline-block text-xs py-[1px] px-[5px] rounded-[4px] bg-blue/20 text-blue-2 font-semibold shrink-0 mr-1">정규식</span>
                  <span class="text-red font-mono text-base whitespace-pre overflow-hidden text-ellipsis">{{ rule.old_text }}</span>
                  <span
                      v-if="rule.conflicts?.length > 0"
                      class="text-amber shrink-0 cursor-help flex items-center"
                      :title="`충돌 규칙 ID: ${rule.conflicts.join(', ')}`"
                  >
                    <TriangleAlert :size="14"/>
                  </span>
                </div>

                <!-- 화살표 -->
                <div class="flex items-center justify-center">
                  <span class="text-ink-5 shrink-0 text-base">→</span>
                </div>

                <!-- 바꿀 텍스트 -->
                <div class="flex items-center gap-1.5 overflow-hidden">
                  <span class="text-green font-mono text-base whitespace-pre overflow-hidden text-ellipsis">{{ rule.new_text || '(빈 문자열)' }}</span>
                </div>

                <!-- 활성화 토글 -->
                <div class="flex items-center justify-center">
                  <button
                      class="text-xs font-bold py-[3px] px-2 rounded-[5px] border-none cursor-pointer transition-colors"
                      :class="rule.enabled ? 'bg-green/[0.15] text-green' : 'bg-ink-5/[0.15] text-ink-5'"
                      @click="toggleEnabled(rule)"
                      :title="rule.enabled ? '비활성화' : '활성화'"
                  >
                    {{ rule.enabled ? 'ON' : 'OFF' }}
                  </button>
                </div>

                <!-- 액션 -->
                <div class="flex items-center justify-end gap-1">
                  <button
                      class="inline-flex items-center justify-center w-7 h-7 rounded-[5px] border-none bg-transparent text-ink-5 cursor-pointer transition-[background-color,color] shrink-0 hover:bg-raised hover:text-ink-2"
                      @click="startEdit(rule)" title="편집"
                  >
                    <Pencil :size="15"/>
                  </button>
                  <button
                      class="inline-flex items-center justify-center w-7 h-7 rounded-[5px] border-none bg-transparent cursor-pointer transition-[background-color,color] shrink-0 hover:bg-red/10"
                      @click="deleteRule(rule.id)" title="삭제"
                  >
                    <Trash2 :size="15" class="text-red/70"/>
                  </button>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>

      <!-- ─── Step 2: 적용 범위 선택 ───────────────────────────── -->
      <div v-if="step === 2">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 2. 적용 범위 선택</h3>
        <p class="text-base text-ink-5 m-0 mb-6">텍스트를 치환할 생기부 범위를 선택하세요.</p>

        <!-- 모드 카드 2개: 전체 영역 / 특정 영역 -->
        <div class="grid grid-cols-2 gap-3 mb-6">
          <div
              class="relative border-2 rounded-[10px] py-[18px] px-5 cursor-pointer flex flex-col gap-1.5 transition-[border-color,background-color] duration-200"
              :class="scopeMode === 'all' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="scopeMode = 'all'"
          >
            <Check v-if="scopeMode === 'all'" :size="14" class="absolute top-2.5 right-3 text-blue-2"/>
            <span class="text-base font-semibold text-ink">전체 영역</span>
            <span class="text-sm text-ink-5">모든 생기부 기록에 치환을 적용합니다</span>
          </div>
          <div
              class="relative border-2 rounded-[10px] py-[18px] px-5 cursor-pointer flex flex-col gap-1.5 transition-[border-color,background-color] duration-200"
              :class="scopeMode === 'specific' ? 'border-blue/70 bg-blue/[0.06]' : 'border-line hover:border-blue/40 hover:bg-blue/[0.03]'"
              @click="scopeMode = 'specific'"
          >
            <Check v-if="scopeMode === 'specific'" :size="14" class="absolute top-2.5 right-3 text-blue-2"/>
            <span class="text-base font-semibold text-ink">특정 영역</span>
            <span class="text-sm text-ink-5">치환을 적용할 영역을 직접 선택합니다</span>
          </div>
        </div>

        <!-- 개별 영역 카드 그리드 -->
        <div
            class="mb-6 transition-opacity"
            :class="{ 'opacity-35 pointer-events-none': scopeMode !== 'specific' }"
        >
          <p class="text-sm text-ink-5 m-0 mb-3">치환을 적용할 영역 선택</p>
          <p v-if="areaStore.areas.length === 0" class="text-sm text-line-2">등록된 영역이 없습니다.</p>
          <div v-else class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(200px, 1fr))">
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
      </div>

      <!-- ─── Step 3: 미리보기 및 적용 ─────────────────────────── -->
      <div v-if="step === 3">
        <h3 class="text-lg font-bold text-ink m-0 mb-1.5">Step 3. 미리보기 및 적용</h3>
        <p class="text-base text-ink-5 m-0 mb-6">변경될 항목을 미리 확인한 후 치환을 적용하세요.</p>

        <!-- 버튼 행 -->
        <div class="flex gap-2 mb-4">
          <button
              class="inline-flex items-center gap-[7px] py-[9px] px-5 rounded-lg border-none bg-blue/25 text-blue-2 text-base cursor-pointer transition-colors enabled:hover:bg-blue/40 disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="isPreviewing"
              @click="runPreview"
          >
            <Eye :size="15"/>
            {{ isPreviewing ? '미리보기 중...' : '미리보기' }}
          </button>

          <button
              class="inline-flex items-center gap-[7px] py-[9px] px-[22px] rounded-lg border-none bg-green/[0.18] text-green text-base font-semibold cursor-pointer transition-colors enabled:hover:bg-green/[0.28] disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="isApplying || previewItems.length === 0"
              @click="runApply"
          >
            <Play :size="15"/>
            {{ isApplying ? '적용 중...' : `적용 (${previewItems.length}건)` }}
          </button>
        </div>

        <!-- 스냅샷 권장 안내 -->
        <div class="flex items-center gap-1.5 text-amber text-sm mt-3 mb-3 py-[9px] px-3 bg-amber/[0.06] rounded-[6px] border border-amber/15">
          <TriangleAlert :size="14"/>
          적용 전 스냅샷 생성을 권장합니다. (사이드바 하단 → 스냅샷)
        </div>

        <!-- 오류 -->
        <div v-if="previewError" class="msg-error mb-4">{{ previewError }}</div>

        <!-- diff 목록 -->
        <div v-if="previewItems.length > 0" class="bg-surface border border-line rounded-[10px] p-4">
          <div class="flex items-center justify-between mb-3">
            <span class="text-lg font-semibold text-ink-4 uppercase tracking-[0.05em]">
              변경 예정 항목 {{ previewItems.length }}건
              <span v-if="previewItems.length > PREVIEW_LIMIT && !showAllPreview" class="text-ink-5 text-sm font-normal normal-case tracking-normal">
                ({{ PREVIEW_LIMIT }}건만 표시)
              </span>
            </span>
          </div>

          <div v-for="item in visiblePreviewItems" :key="`${item.activity_id}-${item.student_id}`"
               class="py-3 border-b border-line last:border-b-0">
            <div class="flex items-center gap-1.5 mb-1.5">
              <span class="text-sm text-ink-5">{{ item.student_name }}</span>
              <span class="text-line-2">/</span>
              <span class="text-sm text-ink-5">{{ item.activity_name }}</span>
            </div>
            <DiffView :before="item.original" :after="item.result"/>
          </div>

          <button
              v-if="previewItems.length > PREVIEW_LIMIT && !showAllPreview"
              class="inline-flex items-center gap-[5px] py-1.5 px-3 rounded-[6px] border border-line bg-transparent text-ink-5 text-sm cursor-pointer transition-colors hover:bg-line hover:text-ink-4 mt-3"
              @click="showAllPreview = true"
          >
            전체 {{ previewItems.length }}건 모두 보기
          </button>
        </div>

        <!-- 0건 결과 -->
        <div v-else-if="hasRanPreview && previewItems.length === 0 && applyResult === null && !previewError"
             class="flex flex-col items-center gap-2 py-9 px-[18px] text-center border border-dashed border-green/20 bg-green/[0.03] rounded-lg">
          <SearchX :size="36" class="text-green/40"/>
          <p class="text-base font-semibold text-green m-0">변경될 항목이 없습니다</p>
          <p class="text-sm text-ink-5 m-0">활성화된 규칙과 일치하는 텍스트를 찾지 못했습니다.</p>
        </div>

        <!-- 초기 안내 -->
        <div v-else-if="!hasRanPreview && !isPreviewing && applyResult === null && !previewError"
             class="text-line-2 text-sm py-9 px-[18px] text-center border border-dashed border-line rounded-lg">
          미리보기를 실행하면 변경될 항목이 표시됩니다.
        </div>
      </div>

      <!-- ─── Step 4: 완료 ──────────────────────────────────────── -->
      <div v-if="step === 4">
        <div v-if="applyResult" class="flex flex-col items-center gap-4 py-12">
          <div class="text-[40px] text-green">✓</div>
          <p class="text-xl font-bold text-ink m-0">텍스트 치환 완료</p>
          <div class="flex gap-8">
            <div class="flex flex-col items-center gap-1">
              <span class="text-[28px] font-bold text-blue-2">{{ applyResult.changed_count }}건 적용 완료</span>
              <span class="text-sm text-ink-5">(전체 {{ applyResult.total_count }}건 중)</span>
            </div>
          </div>
          <div class="flex gap-2.5 mt-2">
            <button
                class="py-[9px] px-6 bg-transparent border border-line rounded-lg text-ink-5 text-base cursor-pointer transition-colors hover:bg-line hover:text-ink-3"
                @click="resetWizard"
            >새로 치환하기</button>
          </div>
        </div>

        <div v-else-if="applyError" class="msg-error">
          {{ applyError }}
        </div>
      </div>

    </WizardLayout>

  </div>
</template>
