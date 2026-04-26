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
  <div class="section">

    <!-- 헤더 -->
    <div class="toolbar">
      <div class="section-header">
        <h2 class="section-title">텍스트 치환(Replace)</h2>
        <p class="section-desc">학교생활기록부 문장의 특수문자, 텍스트 등을 일괄 교체합니다.</p>
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
    <!-- 본문 -->

      <!-- ─── Step 1: 규칙 관리 ─────────────────────────────── -->
      <div v-if="step === 1" class="step-content">
        <div class="step-header">
          <h3 class="step-title">Step 1. 텍스트 치환 규칙</h3>
          <p class="step-desc">찾아 바꿀 텍스트 규칙을 관리합니다. <u>우선순위 숫자가 작은 규칙부터 먼저 실행</u>됩니다.</p>
        </div>

        <div class="panel">
          <div class="panel-head">
            <span class="panel-title">치환 규칙 목록</span>
            <button class="btn-sm btn-add" @click="showAddForm = !showAddForm">
              <Plus :size="14"/>
              규칙 추가
            </button>
          </div>

          <!-- 추가 폼 -->
          <div v-if="showAddForm" class="add-form">
            <input
                v-model="newRule.oldText"
                class="input-sm"
                placeholder="찾을 텍스트"
                @keydown.enter="submitAdd"
            />
            <span class="arrow-label">→</span>
            <input
                v-model="newRule.newText"
                class="input-sm"
                placeholder="바꿀 텍스트"
                @keydown.enter="submitAdd"
            />
            <input
                v-model.number="newRule.priority"
                type="number"
                class="input-sm input-priority"
                placeholder="우선순위"
                @keydown.enter="submitAdd"
            />
            <label class="regex-toggle-label">
              <input type="checkbox" v-model="newRule.isRegex"/>
              정규식
            </label>
            <button class="btn-icon btn-confirm" @click="submitAdd" title="추가">
              <Check :size="14"/>
            </button>
            <button class="btn-icon btn-cancel" @click="showAddForm = false; addError = ''" title="취소">
              <X :size="14"/>
            </button>
            <p v-if="addError" class="error-msg">{{ addError }}</p>
          </div>

          <p v-if="ruleStore.error" class="error-msg mt-2">{{ ruleStore.error }}</p>
          <p v-if="operationError" class="error-msg mt-2">{{ operationError }}</p>

          <!-- 규칙 테이블 -->
          <div v-if="ruleStore.rules.length === 0 && !ruleStore.loading" class="empty-state">
            규칙이 없습니다. 추가 버튼으로 새 규칙을 만드세요.
          </div>

          <div v-else class="rule-table">
            <div class="rule-header-row">
              <span class="col-priority">우선순위</span>
              <span class="col-old">찾을 텍스트</span>
              <span class="col-arrow">→</span>
              <span class="col-new">바꿀 텍스트</span>
              <span class="col-toggle">활성화</span>
              <span class="col-actions">편집</span>
            </div>

            <div
                v-for="rule in ruleStore.rules"
                :key="rule.id"
                :class="['rule-row', !rule.enabled && 'rule-row--disabled']"
            >
              <!-- 인라인 편집 모드 -->
              <template v-if="editingId === rule.id">
                <div class="edit-row">
                  <input v-model.number="editForm.priority" type="number" class="input-sm input-priority" title="우선순위"/>
                  <input v-model="editForm.oldText" class="input-sm" placeholder="찾을 텍스트"/>
                  <span class="arrow-label">→</span>
                  <input v-model="editForm.newText" class="input-sm" placeholder="바꿀 텍스트"/>
                  <label class="regex-toggle-label">
                    <input type="checkbox" v-model="editForm.isRegex"/>
                    정규식
                  </label>
                  <button class="btn-icon btn-confirm" @click="commitEdit(rule)" title="저장">
                    <Check :size="15"/>
                  </button>
                  <button class="btn-icon btn-cancel" @click="cancelEdit" title="취소">
                    <X :size="15"/>
                  </button>
                </div>
                <p v-if="editError" class="error-msg">{{ editError }}</p>
              </template>

              <!-- 표시 모드 -->
              <template v-else>
                <div class="col-priority priority-ctrl">
                  <button class="btn-icon-tiny" :disabled="isAdjusting" @click="adjustPriority(rule, -1)" title="우선순위 높이기">
                    <ChevronLeft :size="14"/>
                  </button>
                  <span class="priority-val">{{ rule.priority }}</span>
                  <button class="btn-icon-tiny" :disabled="isAdjusting" @click="adjustPriority(rule, 1)" title="우선순위 낮추기">
                    <ChevronRight :size="14"/>
                  </button>
                </div>

                <div class="col-old">
                  <span v-if="rule.is_regex" class="badge-regex">정규식</span>
                  <span class="old-text">{{ rule.old_text }}</span>
                  <span
                      v-if="rule.conflicts?.length > 0"
                      class="conflict-badge"
                      :title="`충돌 규칙 ID: ${rule.conflicts.join(', ')}`"
                  >
                    <TriangleAlert :size="14"/>
                  </span>
                </div>

                <div class="col-arrow">
                  <span class="arrow-label">→</span>
                </div>

                <div class="col-new">
                  <span class="new-text">{{ rule.new_text || '(빈 문자열)' }}</span>
                </div>

                <div class="col-toggle">
                  <button
                      :class="['toggle-btn', rule.enabled ? 'toggle-btn--on' : 'toggle-btn--off']"
                      @click="toggleEnabled(rule)"
                      :title="rule.enabled ? '비활성화' : '활성화'"
                  >
                    {{ rule.enabled ? 'ON' : 'OFF' }}
                  </button>
                </div>

                <div class="col-actions action-btns">
                  <button class="btn-icon" @click="startEdit(rule)" title="편집">
                    <Pencil :size="15"/>
                  </button>
                  <button class="btn-icon" @click="deleteRule(rule.id)" title="삭제">
                    <Trash2 :size="15" color="rgba(248, 113, 113, 0.7)"/>
                  </button>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>

      <!-- ─── Step 2: 적용 범위 선택 ───────────────────────────── -->
      <div v-if="step === 2" class="step-content">
        <div class="step-header">
          <h3 class="step-title">Step 2. 적용 범위 선택</h3>
          <p class="step-desc">텍스트를 치환할 생기부 범위를 선택하세요.</p>
        </div>

        <!-- 모드 카드 2개: 전체 영역 / 특정 영역 -->
        <div class="scope-mode-cards">
          <div
              class="scope-mode-card"
              :class="{ 'scope-mode-card--on': scopeMode === 'all' }"
              @click="scopeMode = 'all'"
          >
            <Check v-if="scopeMode === 'all'" :size="14" class="scope-mode-check"/>
            <span class="scope-mode-title">전체 영역</span>
            <span class="scope-mode-desc">모든 생기부 기록에 치환을 적용합니다</span>
          </div>
          <div
              class="scope-mode-card"
              :class="{ 'scope-mode-card--on': scopeMode === 'specific' }"
              @click="scopeMode = 'specific'"
          >
            <Check v-if="scopeMode === 'specific'" :size="14" class="scope-mode-check"/>
            <span class="scope-mode-title">특정 영역</span>
            <span class="scope-mode-desc">치환을 적용할 영역을 직접 선택합니다</span>
          </div>
        </div>

        <!-- 개별 영역 카드 그리드 (scopeMode='specific'일 때 활성) -->
        <div class="area-section" :class="{ 'area-section--disabled': scopeMode !== 'specific' }">
          <p class="area-section-label">치환을 적용할 영역 선택</p>
          <p v-if="areaStore.areas.length === 0" class="empty-hint">등록된 영역이 없습니다.</p>
          <div v-else class="area-cards">
            <div
                v-for="area in areaStore.areas"
                :key="area.id"
                class="area-card"
                :class="{ 'area-card--selected': isAreaSelected(area.id) }"
                @click="toggleArea(area.id)"
            >
              <Check v-if="isAreaSelected(area.id)" :size="14" class="area-card-check"/>
              <span class="area-card-name">{{ area.name }}</span>
              <span class="area-card-meta">활동 {{ area.activities.length }}개</span>
            </div>
          </div>
          <p v-if="scopeMode === 'specific' && selectedAreaIds.length > 0" class="area-summary">
            {{ selectedAreaIds.length }}개 영역 선택됨
          </p>
        </div>
      </div>

      <!-- ─── Step 3: 미리보기 및 적용 ─────────────────────────── -->
      <div v-if="step === 3" class="step-content">
        <div class="step-header">
          <h3 class="step-title">Step 3. 미리보기 및 적용</h3>
          <p class="step-desc">변경될 항목을 미리 확인한 후 치환을 적용하세요.</p>
        </div>

        <!-- 미리보기 버튼 -->
        <div class="action-row">
          <button
              class="btn-primary"
              :disabled="isPreviewing"
              @click="runPreview"
          >
            <Eye :size="15"/>
            {{ isPreviewing ? '미리보기 중...' : '미리보기' }}
          </button>

          <button
              class="btn-apply"
              :disabled="isApplying || previewItems.length === 0"
              @click="runApply"
          >
            <Play :size="15"/>
            {{ isApplying ? '적용 중...' : `적용 (${previewItems.length}건)` }}
          </button>
        </div>

        <!-- 스냅샷 권장 안내 -->
        <div class="snapshot-notice">
          <TriangleAlert :size="14"/>
          적용 전 스냅샷 생성을 권장합니다. (사이드바 하단 → 스냅샷)
        </div>

        <!-- 오류 -->
        <div v-if="previewError" class="error-box">{{ previewError }}</div>

        <!-- diff 목록 -->
        <div v-if="previewItems.length > 0" class="panel">
          <div class="panel-head">
            <span class="panel-title">
              변경 예정 항목 {{ previewItems.length }}건
              <span v-if="previewItems.length > PREVIEW_LIMIT && !showAllPreview" class="hint-text">
                ({{ PREVIEW_LIMIT }}건만 표시)
              </span>
            </span>
          </div>

          <div v-for="item in visiblePreviewItems" :key="`${item.activity_id}-${item.student_id}`" class="diff-item">
            <div class="diff-meta">
              <span class="diff-label">{{ item.student_name }}</span>
              <span class="diff-sep">/</span>
              <span class="diff-label">{{ item.activity_name }}</span>
            </div>
            <DiffView :before="item.original" :after="item.result"/>
          </div>

          <button
              v-if="previewItems.length > PREVIEW_LIMIT && !showAllPreview"
              class="btn-sm btn-ghost mt-3"
              @click="showAllPreview = true"
          >
            전체 {{ previewItems.length }}건 모두 보기
          </button>

        </div>

        <!-- 0건 결과 -->
        <div v-else-if="hasRanPreview && previewItems.length === 0 && applyResult === null && !previewError"
             class="empty-preview empty-preview--no-result">
          <SearchX :size="36" class="empty-preview__icon"/>
          <p class="empty-preview__title">변경될 항목이 없습니다</p>
          <p class="empty-preview__desc">활성화된 규칙과 일치하는 텍스트를 찾지 못했습니다.</p>
        </div>

        <!-- 초기 안내 -->
        <div v-else-if="!hasRanPreview && !isPreviewing && applyResult === null && !previewError"
             class="empty-preview">
          미리보기를 실행하면 변경될 항목이 표시됩니다.
        </div>

      </div>

      <div v-if="step === 4">
        <!-- 적용 결과 -->
        <div v-if="applyResult" class="result-box">
          <div class="result-check">✓</div>
          <p class="result-title">텍스트 치환 완료</p>
          <div class="result-stats">
            <div class="stat-item">
              <span class="stat-val">{{ applyResult.changed_count }}건 적용 완료</span>
              <span class="stat-label">(전체 {{ applyResult.total_count }}건 중)</span>
            </div>
          </div>
          <div class="result-actions">
            <button class="btn-reset" @click="resetWizard">새로 치환하기</button>
          </div>
        </div>

        <div v-else-if="applyError" class="error-box">
          {{ applyError }}
        </div>

      </div>

    </WizardLayout>

  </div>
</template>

<style scoped>
/* ── 레이아웃 ─────────────────────────────────────────────── */
.section {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  box-sizing: border-box;
}

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

.step-content {
}

.step-header {
  margin-bottom: 24px;
}

.step-title {
  font-size: 18px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0 0 6px;
}

.step-desc {
  font-size: 15px;
  color: #7c8db5;
  margin: 0 0 24px;
}

/* ── Step 2: 범위 선택 카드 ──────────────────────────────── */
.scope-mode-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-bottom: 24px;
}

.scope-mode-card {
  position: relative;
  border: 2px solid #1a2035;
  border-radius: 10px;
  padding: 18px 20px;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 6px;
  transition: border-color 0.2s, background-color 0.2s;
}

.scope-mode-card:hover {
  border-color: rgba(59, 91, 219, 0.4);
  background-color: rgba(59, 91, 219, 0.03);
}

.scope-mode-card--on {
  border-color: rgba(59, 91, 219, 0.7);
  background-color: rgba(59, 91, 219, 0.06);
}

.scope-mode-check {
  position: absolute;
  top: 10px;
  right: 12px;
  color: #7ba8f0;
}

.scope-mode-title {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
}

.scope-mode-desc {
  font-size: 13px;
  color: var(--clr-text-subtle);
}

.area-section {
  margin-bottom: 24px;
  transition: opacity 0.2s;
}

.area-section--disabled {
  opacity: 0.35;
  pointer-events: none;
}

.area-section-label {
  font-size: 13px;
  color: #7c8db5;
  margin: 0 0 12px;
}

.area-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}

.area-card {
  position: relative;
  border: 2px solid #1a2035;
  border-radius: 10px;
  padding: 16px 20px;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.area-card:hover {
  border-color: rgba(59, 91, 219, 0.4);
  background-color: rgba(59, 91, 219, 0.03);
}

.area-card--selected {
  border-color: rgba(59, 91, 219, 0.7);
  background-color: rgba(59, 91, 219, 0.06);
}

.area-card-check {
  position: absolute;
  top: 10px;
  right: 12px;
  color: #7ba8f0;
}

.area-card-name {
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
}

.area-card-meta {
  font-size: 13px;
  color: var(--clr-text-subtle);
}

.area-summary {
  font-size: 13px;
  color: #7c8db5;
  margin-top: 12px;
}

.empty-hint {
  font-size: 13px;
  color: #4a5568;
}

/* ── 패널 ─────────────────────────────────────────────────── */
.panel {
  background-color: #0d1120;
  border: 1px solid #1a2035;
  border-radius: 10px;
  padding: 16px;
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.panel-title {
  font-size: 18px;
  font-weight: 600;
  color: #94a3b8;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

/* ── 추가 폼 ─────────────────────────────────────────────── */
.add-form {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding: 12px;
  background: #111827;
  border-radius: 8px;
  margin-bottom: 10px;
}

/* ── 규칙 테이블 ─────────────────────────────────────────── */
.rule-table {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* 6열: 순위 | 찾을 텍스트 | → | 바꿀 텍스트 | 활성 | 액션 */
.rule-header-row,
.rule-row {
  display: grid;
  grid-template-columns: 72px 1fr 36px 1fr 64px 76px;
  gap: 8px;
  font-size: 18px;
  align-items: center;
}

.rule-header-row {
  padding: 4px 10px;
  font-size: 14px;
  color: #6b8ab5;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.rule-header-row .col-priority,
.rule-header-row .col-toggle,
.rule-header-row .col-actions {
  display: flex;
  justify-content: center;
  text-align: center;
}

.rule-row {
  padding: 8px 10px;
  border-radius: 6px;
  transition: background-color 0.1s;
}

.rule-row:hover {
  background-color: #151c2e;
}

.rule-row--disabled {
  opacity: 0.45;
}

.edit-row {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

/* 우선순위 컨트롤 */
.priority-ctrl {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.priority-val {
  font-size: 14px;
  color: #64748b;
  min-width: 26px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}

/* 텍스트 열 */
.col-old,
.col-new {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
}

.col-old {
  justify-content: flex-end;
}

.old-text {
  color: #f87171;
  font-family: monospace;
  font-size: 20px;
  white-space: pre;
  overflow: hidden;
  text-overflow: ellipsis;
}

.new-text {
  color: #4ade80;
  font-family: monospace;
  font-size: 20px;
  white-space: pre;
  overflow: hidden;
  text-overflow: ellipsis;
}

.col-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
}

.arrow-label {
  color: #475569;
  flex-shrink: 0;
  font-size: 16px;
}

.conflict-badge {
  color: #f59e0b;
  flex-shrink: 0;
  cursor: help;
  display: flex;
  align-items: center;
}

.badge-regex {
  display: inline-block;
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 4px;
  background: #dbeafe;
  color: #1d4ed8;
  font-weight: 600;
  flex-shrink: 0;
  margin-right: 4px;
}

.regex-toggle-label {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: #475569;
  white-space: nowrap;
  cursor: pointer;
  flex-shrink: 0;
}

.action-btns {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
}

/* 토글 버튼 */
.toggle-btn {
  font-size: 12px;
  font-weight: 700;
  padding: 3px 8px;
  border-radius: 5px;
  border: none;
  cursor: pointer;
  transition: background-color 0.15s;
}

.toggle-btn--on {
  background-color: rgba(74, 222, 128, 0.15);
  color: #4ade80;
}

.toggle-btn--off {
  background-color: rgba(100, 116, 139, 0.15);
  color: #475569;
}

/* ── 버튼류 ──────────────────────────────────────────────── */
.btn-sm {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 6px 12px;
  border-radius: 6px;
  border: none;
  font-size: 14px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-add {
  background-color: rgba(59, 91, 219, 0.2);
  color: #93c5fd;
}

.btn-add:hover {
  background-color: rgba(59, 91, 219, 0.35);
}

.btn-ghost {
  background: none;
  color: #64748b;
  border: 1px solid #1a2035;
}

.btn-ghost:hover {
  background-color: #1a2035;
  color: #94a3b8;
}

.btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 5px;
  border: none;
  background: none;
  color: #64748b;
  cursor: pointer;
  transition: background-color 0.1s, color 0.1s;
  flex-shrink: 0;
}

.btn-icon:hover {
  background-color: #1e293b;
  color: #c8d8f0;
}

.btn-icon-tiny {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 3px;
  border: none;
  background: none;
  color: #475569;
  cursor: pointer;
}

.btn-icon-tiny:hover {
  background-color: #1e293b;
  color: #94a3b8;
}

.btn-confirm {
  color: #4ade80 !important;
}

.btn-confirm:hover {
  background-color: rgba(74, 222, 128, 0.1) !important;
}

.btn-cancel {
  color: #f87171 !important;
}

.btn-cancel:hover {
  background-color: rgba(248, 113, 113, 0.1) !important;
}

.btn-danger:hover {
  background-color: rgba(248, 113, 113, 0.12) !important;
  color: #f87171 !important;
}

.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 20px;
  border-radius: 8px;
  border: none;
  background-color: rgba(59, 91, 219, 0.25);
  color: #93c5fd;
  font-size: 15px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-primary:hover:not(:disabled) {
  background-color: rgba(59, 91, 219, 0.4);
}

.btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-apply {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 22px;
  border-radius: 8px;
  border: none;
  background-color: rgba(74, 222, 128, 0.18);
  color: #4ade80;
  font-size: 15px;
  cursor: pointer;
  font-weight: 600;
  transition: background-color 0.15s;
}

.btn-apply:hover:not(:disabled) {
  background-color: rgba(74, 222, 128, 0.28);
}

.btn-apply:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* ── 입력 ─────────────────────────────────────────────────── */
.input-sm {
  background-color: #111827;
  border: 1px solid #1e293b;
  border-radius: 6px;
  color: #c8d8f0;
  font-size: 18px;
  padding: 6px 10px;
  outline: none;
  flex: 1;
  min-width: 80px;
}

.input-sm:focus {
  border-color: #3b5bdb;
}

.input-priority {
  flex: 0 0 68px;
  min-width: 68px;
}

/* ── diff 항목 ───────────────────────────────────────────── */
.diff-item {
  padding: 12px 0;
  border-bottom: 1px solid #1a2035;
}

.diff-item:last-child {
  border-bottom: none;
}

.diff-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.diff-label {
  font-size: 14px;
  color: #64748b;
}

.diff-sep {
  color: #334155;
}

/* ── 기타 ─────────────────────────────────────────────────── */
.action-row {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.mt-2 {
  margin-top: 8px;
}

.mt-3 {
  margin-top: 12px;
}

.error-msg {
  color: #f87171;
  font-size: 14px;
  margin: 4px 0 0;
  width: 100%;
}

.error-box {
  background-color: rgba(248, 113, 113, 0.08);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: 8px;
  color: #f87171;
  padding: 12px 16px;
  font-size: 14px;
  margin-bottom: 16px;
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

.result-filename {
  font-size: 14px;
  color: var(--clr-text-subtle);
  margin: 0;
}

.result-actions {
  display: flex;
  gap: 10px;
  margin-top: 8px;
}

.btn-reveal {
  padding: 9px 24px;
  background: rgba(59, 91, 219, 0.12);
  border: 1px solid rgba(59, 91, 219, 0.35);
  border-radius: 8px;
  color: #7ba8f0;
  font-size: 15px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.btn-reveal:hover {
  background: rgba(59, 91, 219, 0.22);
  color: #93c5fd;
}

.btn-reset {
  padding: 9px 24px;
  margin-top: 0;
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

.empty-state {
  color: #334155;
  font-size: 14px;
  padding: 14px 0;
  text-align: center;
}

.empty-preview {
  color: #334155;
  font-size: 14px;
  padding: 36px 18px;
  text-align: center;
  border: 1px dashed #1a2035;
  border-radius: 8px;
}

.empty-preview--no-result {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 36px 18px;
  border-color: #1e3a2f;
  background-color: rgba(74, 222, 128, 0.03);
}

.empty-preview__icon {
  color: #2d6a4a;
}

.empty-preview__title {
  font-size: 15px;
  font-weight: 600;
  color: #4ade80;
  margin: 0;
}

.empty-preview__desc {
  font-size: 14px;
  color: #475569;
  margin: 0;
}

.snapshot-notice {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #f59e0b;
  font-size: 14px;
  margin-top: 12px;
  margin-bottom: 12px;
  padding: 9px 12px;
  background-color: rgba(245, 158, 11, 0.06);
  border-radius: 6px;
  border: 1px solid rgba(245, 158, 11, 0.15);
}

.hint-text {
  color: #475569;
  font-size: 14px;
  font-weight: normal;
  text-transform: none;
  letter-spacing: 0;
}

.col-priority {
  display: flex;
  align-items: center;
}

.col-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
}

.col-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
}
</style>
