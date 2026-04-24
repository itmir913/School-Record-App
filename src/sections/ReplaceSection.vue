<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useReplaceRuleStore } from '../stores/replaceRule'
import { useAreaStore } from '../stores/area'
import DiffView from '../components/DiffView.vue'
import {
  Plus, Trash2, Pencil, Check, X, ChevronUp, ChevronDown,
  TriangleAlert, Eye, Play,
} from 'lucide-vue-next'

const ruleStore = useReplaceRuleStore()
const areaStore = useAreaStore()

// ── 범위 선택 ──────────────────────────────────────────────
const scope = ref('all')          // 'all' | 'area'
const selectedAreaId = ref(null)

// ── 규칙 인라인 편집 ───────────────────────────────────────
const editingId = ref(null)
const editForm = ref({ oldText: '', newText: '', priority: 0 })

function startEdit(rule) {
  editingId.value = rule.id
  editForm.value = { oldText: rule.old_text, newText: rule.new_text, priority: rule.priority }
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
    )
    editingId.value = null
  } catch (e) {
    editError.value = e?.toString() ?? '수정 실패'
  }
}

async function toggleEnabled(rule) {
  try {
    await ruleStore.updateRule(
      rule.id, rule.old_text, rule.new_text, !rule.enabled, rule.priority,
    )
  } catch (e) {
    ruleStore.error = e?.toString() ?? '수정 실패'
  }
}

async function adjustPriority(rule, delta) {
  const newPriority = rule.priority + delta
  try {
    await ruleStore.updateRule(
      rule.id, rule.old_text, rule.new_text, rule.enabled, newPriority,
    )
  } catch (e) {
    ruleStore.error = e?.toString() ?? '수정 실패'
  }
}

async function deleteRule(id) {
  try {
    await ruleStore.deleteRule(id)
  } catch (e) {
    ruleStore.error = e?.toString() ?? '삭제 실패'
  }
}

// ── 규칙 추가 폼 ───────────────────────────────────────────
const showAddForm = ref(false)
const newRule = ref({ oldText: '', newText: '', priority: 0 })
const addError = ref('')
const editError = ref('')

async function submitAdd() {
  addError.value = ''
  try {
    await ruleStore.createRule(newRule.value.oldText, newRule.value.newText, newRule.value.priority)
    newRule.value = { oldText: '', newText: '', priority: 0 }
    showAddForm.value = false
  } catch (e) {
    addError.value = e?.toString() ?? '추가 실패'
  }
}

// ── 미리보기 ───────────────────────────────────────────────
const previewItems = ref([])
const previewError = ref('')
const isPreviewing = ref(false)
const PREVIEW_LIMIT = 50
const showAllPreview = ref(false)

const visiblePreviewItems = computed(() =>
  showAllPreview.value ? previewItems.value : previewItems.value.slice(0, PREVIEW_LIMIT),
)

async function runPreview() {
  previewError.value = ''
  previewItems.value = []
  applyResult.value = null
  isPreviewing.value = true
  try {
    previewItems.value = await ruleStore.previewReplace(
      scope.value,
      scope.value === 'area' ? selectedAreaId.value : null,
    )
    showAllPreview.value = false
  } catch (e) {
    previewError.value = e?.toString() ?? '미리보기 실패'
  } finally {
    isPreviewing.value = false
  }
}

// ── 적용 ──────────────────────────────────────────────────
const applyResult = ref(null)
const applyError = ref('')
const isApplying = ref(false)

async function runApply() {
  applyError.value = ''
  isApplying.value = true
  try {
    applyResult.value = await ruleStore.applyReplace(
      scope.value,
      scope.value === 'area' ? selectedAreaId.value : null,
    )
    previewItems.value = []
  } catch (e) {
    applyError.value = e?.toString() ?? '적용 실패'
  } finally {
    isApplying.value = false
  }
}

// ── 마운트 ─────────────────────────────────────────────────
onMounted(async () => {
  await Promise.all([ruleStore.fetchRules(), areaStore.fetchAreas()])
})
</script>

<template>
  <div class="section-container">
    <div class="section-header">
      <h2 class="section-title">텍스트 치환</h2>
      <p class="section-desc">생기부 기록에 치환 규칙을 일괄 적용합니다.</p>
    </div>

    <div class="section-body">

      <!-- ── 규칙 관리 ── -->
      <div class="panel">
        <div class="panel-head">
          <span class="panel-title">치환 규칙</span>
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
          <button class="btn-icon btn-confirm" @click="submitAdd" title="추가">
            <Check :size="14"/>
          </button>
          <button class="btn-icon btn-cancel" @click="showAddForm = false; addError = ''" title="취소">
            <X :size="14"/>
          </button>
          <p v-if="addError" class="error-msg">{{ addError }}</p>
        </div>

        <p v-if="ruleStore.error" class="error-msg mt-2">{{ ruleStore.error }}</p>

        <!-- 규칙 테이블 -->
        <div v-if="ruleStore.rules.length === 0 && !ruleStore.loading" class="empty-state">
          규칙이 없습니다. 추가 버튼으로 새 규칙을 만드세요.
        </div>

        <div v-else class="rule-table">
          <div class="rule-header-row">
            <span class="col-priority">우선순위</span>
            <span class="col-rule">규칙</span>
            <span class="col-toggle">활성</span>
            <span class="col-actions"></span>
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
                <button class="btn-icon btn-confirm" @click="commitEdit(rule)" title="저장">
                  <Check :size="14"/>
                </button>
                <button class="btn-icon btn-cancel" @click="cancelEdit" title="취소">
                  <X :size="14"/>
                </button>
              </div>
              <p v-if="editError" class="error-msg">{{ editError }}</p>
            </template>

            <!-- 표시 모드 -->
            <template v-else>
              <div class="col-priority priority-ctrl">
                <button class="btn-icon-tiny" @click="adjustPriority(rule, -1)" title="우선순위 높이기">
                  <ChevronUp :size="12"/>
                </button>
                <span class="priority-val">{{ rule.priority }}</span>
                <button class="btn-icon-tiny" @click="adjustPriority(rule, 1)" title="우선순위 낮추기">
                  <ChevronDown :size="12"/>
                </button>
              </div>

              <div class="col-rule rule-text">
                <span class="old-text">{{ rule.old_text }}</span>
                <span class="arrow-label">→</span>
                <span class="new-text">{{ rule.new_text || '(빈 문자열)' }}</span>
                <span
                  v-if="rule.conflicts.length > 0"
                  class="conflict-badge"
                  :title="`충돌 규칙 ID: ${rule.conflicts.join(', ')}`"
                >
                  <TriangleAlert :size="12"/>
                </span>
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
                  <Pencil :size="13"/>
                </button>
                <button class="btn-icon btn-danger" @click="deleteRule(rule.id)" title="삭제">
                  <Trash2 :size="13"/>
                </button>
              </div>
            </template>
          </div>
        </div>
      </div>

      <!-- ── 적용 범위 ── -->
      <div class="panel">
        <div class="panel-title mb-3">적용 범위</div>
        <div class="scope-row">
          <label class="radio-label">
            <input type="radio" v-model="scope" value="all"/>
            전체 생기부 기록
          </label>
          <label class="radio-label">
            <input type="radio" v-model="scope" value="area"/>
            특정 영역
          </label>
          <select
            v-if="scope === 'area'"
            v-model="selectedAreaId"
            class="select-sm"
          >
            <option :value="null" disabled>영역 선택...</option>
            <option v-for="area in areaStore.areas" :key="area.id" :value="area.id">
              {{ area.name }}
            </option>
          </select>
        </div>
      </div>

      <!-- ── 미리보기 버튼 ── -->
      <div class="action-row">
        <button
          class="btn-primary"
          :disabled="isPreviewing || (scope === 'area' && !selectedAreaId)"
          @click="runPreview"
        >
          <Eye :size="15"/>
          {{ isPreviewing ? '미리보기 중...' : '미리보기' }}
        </button>
      </div>

      <!-- ── diff 표시 ── -->
      <div v-if="previewError" class="error-box">{{ previewError }}</div>

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

        <!-- 스냅샷 권장 안내 -->
        <div class="snapshot-notice">
          <TriangleAlert :size="14"/>
          적용 전 스냅샷 생성을 권장합니다. (사이드바 하단 → 스냅샷)
        </div>

        <div class="action-row mt-2">
          <button
            class="btn-apply"
            :disabled="isApplying"
            @click="runApply"
          >
            <Play :size="15"/>
            {{ isApplying ? '적용 중...' : `적용 (${previewItems.length}건)` }}
          </button>
        </div>
      </div>

      <div v-else-if="!isPreviewing && previewItems.length === 0 && applyResult === null && !previewError" class="empty-preview">
        미리보기를 실행하면 변경될 항목이 표시됩니다.
      </div>

      <!-- ── 적용 결과 ── -->
      <div v-if="applyResult" class="result-box">
        <span class="result-icon">✓</span>
        {{ applyResult.changed_count }}건 적용 완료
        (전체 {{ applyResult.total_count }}건 중)
      </div>

      <div v-if="applyError" class="error-box">{{ applyError }}</div>

    </div>
  </div>
</template>

<style scoped>
.section-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  color: #c8d8f0;
  font-size: 14px;
}

.section-header {
  padding: 24px 28px 16px;
  border-bottom: 1px solid #1a2035;
}

.section-title {
  font-size: 18px;
  font-weight: 600;
  color: #e2e8f0;
  margin: 0 0 4px;
}

.section-desc {
  color: #64748b;
  margin: 0;
  font-size: 13px;
}

.section-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 28px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* 패널 */
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
  font-size: 13px;
  font-weight: 600;
  color: #94a3b8;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.mb-3 { margin-bottom: 12px; }
.mt-2 { margin-top: 8px; }
.mt-3 { margin-top: 12px; }

/* 추가 폼 */
.add-form {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  padding: 10px;
  background: #111827;
  border-radius: 8px;
  margin-bottom: 10px;
}

/* 규칙 테이블 */
.rule-table {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.rule-header-row {
  display: grid;
  grid-template-columns: 80px 1fr 56px 72px;
  gap: 8px;
  padding: 4px 8px;
  font-size: 11px;
  color: #475569;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.rule-row {
  display: grid;
  grid-template-columns: 80px 1fr 56px 72px;
  gap: 8px;
  align-items: center;
  padding: 6px 8px;
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
  gap: 6px;
  flex-wrap: wrap;
}

/* 우선순위 컨트롤 */
.priority-ctrl {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1px;
}

.priority-val {
  font-size: 12px;
  color: #64748b;
  min-width: 20px;
  text-align: center;
}

/* 규칙 텍스트 */
.rule-text {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
}

.old-text {
  color: #f87171;
  font-family: monospace;
  font-size: 13px;
  white-space: pre;
}

.new-text {
  color: #4ade80;
  font-family: monospace;
  font-size: 13px;
  white-space: pre;
}

.arrow-label {
  color: #475569;
  flex-shrink: 0;
  font-size: 13px;
}

.conflict-badge {
  color: #f59e0b;
  flex-shrink: 0;
  cursor: help;
  display: flex;
  align-items: center;
}

.action-btns {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
}

/* 토글 버튼 */
.toggle-btn {
  font-size: 10px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 4px;
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

/* 범위 선택 */
.scope-row {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.radio-label {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  color: #94a3b8;
}

.radio-label input {
  accent-color: #3b5bdb;
}

/* 버튼류 */
.btn-sm {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  border-radius: 6px;
  border: none;
  font-size: 12px;
  cursor: pointer;
  transition: background-color 0.15s;
}

.btn-add {
  background-color: rgba(59, 91, 219, 0.2);
  color: #93c5fd;
}
.btn-add:hover { background-color: rgba(59, 91, 219, 0.35); }

.btn-ghost {
  background: none;
  color: #64748b;
  border: 1px solid #1a2035;
}
.btn-ghost:hover { background-color: #1a2035; color: #94a3b8; }

.btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 5px;
  border: none;
  background: none;
  color: #64748b;
  cursor: pointer;
  transition: background-color 0.1s, color 0.1s;
  flex-shrink: 0;
}
.btn-icon:hover { background-color: #1e293b; color: #c8d8f0; }

.btn-icon-tiny {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 3px;
  border: none;
  background: none;
  color: #475569;
  cursor: pointer;
}
.btn-icon-tiny:hover { background-color: #1e293b; color: #94a3b8; }

.btn-confirm { color: #4ade80 !important; }
.btn-confirm:hover { background-color: rgba(74, 222, 128, 0.1) !important; }

.btn-cancel { color: #f87171 !important; }
.btn-cancel:hover { background-color: rgba(248, 113, 113, 0.1) !important; }

.btn-danger:hover { background-color: rgba(248, 113, 113, 0.12) !important; color: #f87171 !important; }

.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 18px;
  border-radius: 8px;
  border: none;
  background-color: rgba(59, 91, 219, 0.25);
  color: #93c5fd;
  font-size: 14px;
  cursor: pointer;
  transition: background-color 0.15s;
}
.btn-primary:hover:not(:disabled) { background-color: rgba(59, 91, 219, 0.4); }
.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-apply {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 20px;
  border-radius: 8px;
  border: none;
  background-color: rgba(74, 222, 128, 0.18);
  color: #4ade80;
  font-size: 14px;
  cursor: pointer;
  font-weight: 600;
  transition: background-color 0.15s;
}
.btn-apply:hover:not(:disabled) { background-color: rgba(74, 222, 128, 0.28); }
.btn-apply:disabled { opacity: 0.4; cursor: not-allowed; }

/* 입력 */
.input-sm {
  background-color: #111827;
  border: 1px solid #1e293b;
  border-radius: 6px;
  color: #c8d8f0;
  font-size: 13px;
  padding: 5px 8px;
  outline: none;
  flex: 1;
  min-width: 80px;
}
.input-sm:focus { border-color: #3b5bdb; }

.input-priority {
  flex: 0 0 64px;
  min-width: 64px;
}

.select-sm {
  background-color: #111827;
  border: 1px solid #1e293b;
  border-radius: 6px;
  color: #c8d8f0;
  font-size: 13px;
  padding: 5px 8px;
  outline: none;
}
.select-sm:focus { border-color: #3b5bdb; }

/* diff 항목 */
.diff-item {
  padding: 10px 0;
  border-bottom: 1px solid #1a2035;
}
.diff-item:last-child { border-bottom: none; }

.diff-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.diff-label {
  font-size: 12px;
  color: #64748b;
}
.diff-sep { color: #334155; }

/* 기타 */
.action-row {
  display: flex;
  gap: 8px;
}

.error-msg {
  color: #f87171;
  font-size: 12px;
  margin: 4px 0 0;
  width: 100%;
}

.error-box {
  background-color: rgba(248, 113, 113, 0.08);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: 8px;
  color: #f87171;
  padding: 10px 14px;
  font-size: 13px;
}

.result-box {
  background-color: rgba(74, 222, 128, 0.08);
  border: 1px solid rgba(74, 222, 128, 0.2);
  border-radius: 8px;
  color: #4ade80;
  padding: 12px 16px;
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.result-icon { font-size: 16px; }

.empty-state {
  color: #334155;
  font-size: 13px;
  padding: 12px 0;
  text-align: center;
}

.empty-preview {
  color: #334155;
  font-size: 13px;
  padding: 16px;
  text-align: center;
  border: 1px dashed #1a2035;
  border-radius: 8px;
}

.snapshot-notice {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #f59e0b;
  font-size: 12px;
  margin-top: 12px;
  padding: 8px 10px;
  background-color: rgba(245, 158, 11, 0.06);
  border-radius: 6px;
  border: 1px solid rgba(245, 158, 11, 0.15);
}

.hint-text {
  color: #475569;
  font-size: 12px;
  font-weight: normal;
  text-transform: none;
  letter-spacing: 0;
}

.col-priority { display: flex; align-items: center; }
.col-rule { overflow: hidden; }
.col-toggle { display: flex; align-items: center; justify-content: center; }
.col-actions { display: flex; align-items: center; justify-content: flex-end; }
</style>
