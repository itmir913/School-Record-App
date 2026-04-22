<script setup>
import {computed, nextTick, onBeforeUnmount, onMounted, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {ArrowLeftRight, CircleAlert, Minimize2, MousePointerClick, Pin, PinOff} from 'lucide-vue-next'
import {useAreaStore} from '../stores/area'
import CellHistoryModal from '../components/CellHistoryModal.vue'

const areaStore = useAreaStore()

const selectedAreaId = ref(null)
const gridData = ref(null)
const loading = ref(false)
const freezeColumns = ref(true)
const smartScroll = ref(true)
const compactCell = ref(true)
const highlightEmpty = ref(false)
const collapsedActivities = ref(new Set())

function toggleActivity(actId) {
  const next = new Set(collapsedActivities.value)
  if (next.has(actId)) next.delete(actId)
  else next.add(actId)
  collapsedActivities.value = next
}

// 셀별 저장 상태 map: `${activityId}-${studentId}` → 'saving' | 'saved' | null
const savingState = ref(new Map())
// 편집 중인 내용 map
const cellContent = ref(new Map())
// 1초 auto-save debounce 타이머
const debounceTimers = new Map()
// 5분 히스토리 스냅샷 debounce 타이머 (main save 성공 후에만 시작)
const historyTimers = new Map()
// TODO: SettingSection 구현 시 DB 설정값으로 교체
const HISTORY_DEBOUNCE_MS = 5 * 60 * 1000

onMounted(async () => {
  await areaStore.fetchAreas()
})

function clearAllTimers() {
  debounceTimers.forEach(t => clearTimeout(t))
  debounceTimers.clear()
  historyTimers.forEach(t => clearTimeout(t))
  historyTimers.clear()
}

onBeforeUnmount(clearAllTimers)

watch(selectedAreaId, async (id) => {
  clearAllTimers()
  if (!id) {
    gridData.value = null
    return
  }
  loading.value = true
  try {
    gridData.value = await invoke('get_area_grid', {areaId: id})
    // 기존 기록을 cellContent에 세팅
    const map = new Map()
    for (const r of gridData.value.records) {
      map.set(cellKey(r.activity_id, r.student_id), r.content)
    }
    cellContent.value = map
    savingState.value = new Map()
    collapsedActivities.value = new Set()
    if (!compactCell.value) {
      await nextTick()
      document.querySelectorAll('.cell-input').forEach(el => autoResize(el))
    }
  } catch (e) {
    console.error(e)
    gridData.value = null
  } finally {
    loading.value = false
  }
})

function truncateName(name, max = 10) {
  return name.length > max ? name.slice(0, max) + '…' : name
}

function cellKey(activityId, studentId) {
  return `${activityId}-${studentId}`
}

function getCellContent(activityId, studentId) {
  return cellContent.value.get(cellKey(activityId, studentId)) ?? ''
}

function getCellSavingState(activityId, studentId) {
  return savingState.value.get(cellKey(activityId, studentId))
}

function autoResize(el) {
  el.style.height = 'auto'
  el.style.height = el.scrollHeight + 'px'
}

async function toggleCompactCell() {
  compactCell.value = !compactCell.value
  await nextTick()
  document.querySelectorAll('.cell-input').forEach(el => {
    if (compactCell.value) {
      el.style.height = ''
    } else {
      autoResize(el)
    }
  })
}

function onCellInput(activityId, studentId, event) {
  const key = cellKey(activityId, studentId)
  const content = event.target.value
  const map = new Map(cellContent.value)
  map.set(key, content)
  cellContent.value = map
  if (!compactCell.value) autoResize(event.target)

  // debounce 저장
  if (debounceTimers.has(key)) {
    clearTimeout(debounceTimers.get(key))
  }
  const timer = setTimeout(() => saveCell(activityId, studentId, content), 1000)
  debounceTimers.set(key, timer)
}

function onGridWheel(event) {
  const el = event.currentTarget
  if (Math.abs(event.deltaX) > 0) {
    event.preventDefault()
    el.scrollLeft += event.deltaX
    return
  }
  if (!smartScroll.value) {
    if (event.shiftKey) {
      event.preventDefault()
      el.scrollLeft += event.deltaY
    }
    return
  }
  const inFixedArea = event.target.closest('.td-fixed, .th-fixed') !== null
  if (inFixedArea) return
  event.preventDefault()
  el.scrollLeft += event.deltaY
}

function scheduleHistorySnapshot(activityId, studentId) {
  const key = cellKey(activityId, studentId)
  if (historyTimers.has(key)) clearTimeout(historyTimers.get(key))
  const timer = setTimeout(async () => {
    try {
      await invoke('save_history_snapshot', {activityId, studentId, note: null})
    } catch (e) {
      console.error('history snapshot failed:', e)
    }
    historyTimers.delete(key)
  }, HISTORY_DEBOUNCE_MS)
  historyTimers.set(key, timer)
}

async function saveCell(activityId, studentId, content) {
  const key = cellKey(activityId, studentId)
  const stateMap = new Map(savingState.value)
  stateMap.set(key, 'saving')
  savingState.value = stateMap
  try {
    await invoke('upsert_record', {activityId, studentId, content})
    const next = new Map(savingState.value)
    next.set(key, 'saved')
    savingState.value = next
    setTimeout(() => {
      const clear = new Map(savingState.value)
      clear.delete(key)
      savingState.value = clear
    }, 500)
    scheduleHistorySnapshot(activityId, studentId)
  } catch (e) {
    console.error(e)
    const next = new Map(savingState.value)
    next.set(key, 'error')
    savingState.value = next
    setTimeout(() => {
      const clear = new Map(savingState.value)
      clear.delete(key)
      savingState.value = clear
    }, 3000)
  }
}

// 바이트 길이 계산 (UTF-8 기준, 엔터 2바이트)
function byteLength(str) {
  if (!str) return 0;

  // 인자가 숫자인 경우를 대비해 확실하게 문자열로 변환 (방어 코드)
  const safeStr = String(str);

  // 1. 기존 \r 제거 후 모든 \n을 \r\n으로 변환하여 엔터를 2바이트로 처리
  const normalizedStr = safeStr.replace(/\r/g, '').replace(/\n/g, '\r\n');

  // 2. TextEncoder를 통해 바이트 수 계산 (한글 3, 영/숫자/공백 1 자동 적용)
  return new TextEncoder().encode(normalizedStr).length;
}

const byteLimit = computed(() => {
  if (!selectedAreaId.value || !areaStore.areas.length) return null
  const area = areaStore.areas.find(a => a.id === selectedAreaId.value)
  return area ? area.byte_limit : null
})

function isOverLimit(activityId, studentId) {
  if (!byteLimit.value) return false
  const content = getCellContent(activityId, studentId)
  return byteLength(content) > byteLimit.value
}

function studentTotalBytes(studentId) {
  if (!gridData.value) return 0
  let total = 0
  for (const act of gridData.value.activities) {
    total += byteLength(getCellContent(act.id, studentId))
  }
  return total
}

function isStudentOverLimit(studentId) {
  if (!byteLimit.value) return false
  return studentTotalBytes(studentId) > byteLimit.value
}

function isStudentEmpty(studentId) {
  return studentTotalBytes(studentId) === 0
}

// 히스토리 모달
const historyModal = ref(null) // { activityId, studentId, activityName, studentName }

function openHistory(act, student) {
  historyModal.value = {
    activityId: act.id,
    studentId: student.id,
    activityName: act.name,
    studentName: student.name,
  }
}

// 학년+반이 바뀌는 행에 구분선 표시
function isNewGroup(students, index) {
  if (index === 0) return false
  const prev = students[index - 1]
  const curr = students[index]
  return prev.grade !== curr.grade || prev.class_num !== curr.class_num
}
</script>

<template>
  <div class="activity-section-wrapper">
    <div class="section">

      <!-- 상단 컨트롤 -->
      <div class="toolbar">
        <div class="toolbar-left">
          <div class="section-wrap">
            <h2 class="section-title">생기부 작성</h2>
            <div class="section-tips">
            <span class="tip">
              <MousePointerClick :size="12"/>
              헤더 클릭 → 열 접기/펼치기
            </span>
              <span class="tip">
              <ArrowLeftRight :size="12"/>
              스마트스크롤 ON → 활동 열 스크롤 시 가로 이동
            </span>
            </div>
          </div>
        </div>

        <div class="toolbar-right">
          <select
              v-model="selectedAreaId"
              class="area-select"
          >
            <option :value="null" disabled>영역 선택...</option>
            <option
                v-for="area in areaStore.areas"
                :key="area.id"
                :value="area.id"
            >{{ area.name }}
            </option>
          </select>
          <button
              class="btn-freeze"
              :class="freezeColumns ? 'btn-freeze--on' : ''"
              @click="freezeColumns = !freezeColumns"
              title="틀고정 켜기/끄기"
          >
            <Pin v-if="freezeColumns" :size="15"/>
            <PinOff v-else :size="15"/>
            {{ freezeColumns ? '틀고정 ON' : '틀고정 OFF' }}
          </button>
          <button
              class="btn-freeze"
              :class="smartScroll ? 'btn-freeze--on' : ''"
              @click="smartScroll = !smartScroll"
              title="스마트 스크롤: 활동 영역에서 휠 → 좌우 스크롤"
          >
            <ArrowLeftRight :size="15"/>
            {{ smartScroll ? '스마트스크롤 ON' : '스마트스크롤 OFF' }}
          </button>
          <button
              class="btn-freeze"
              :class="compactCell ? 'btn-freeze--on' : ''"
              @click="toggleCompactCell"
              title="셀 높이: 고정(ON) / 자동(OFF)"
          >
            <Minimize2 :size="15"/>
            {{ compactCell ? '셀높이 고정' : '셀높이 자동' }}
          </button>
          <button
              class="btn-freeze"
              :class="highlightEmpty ? 'btn-freeze--on btn-freeze--warn' : ''"
              @click="highlightEmpty = !highlightEmpty"
              title="기록이 없는 학생 행 강조 켜기/끄기"
          >
            <CircleAlert :size="15"/>
            {{ highlightEmpty ? '빈학생 ON' : '빈학생 OFF' }}
          </button>
        </div>
      </div>

      <!-- 빈 상태: 영역 미선택 -->
      <div v-if="!selectedAreaId" class="empty-state">
        <p class="empty-text">상단 드롭다운 메뉴에서 영역(Area)을 선택하세요.</p>
      </div>

      <!-- 로딩 -->
      <div v-else-if="loading" class="empty-state">
        <p class="empty-text">불러오는 중...</p>
      </div>

      <!-- 그리드 없음 (학생 또는 활동 없음) -->
      <div v-else-if="!gridData || gridData.students.length === 0 || gridData.activities.length === 0"
           class="empty-state">
        <p class="empty-text">
          <template v-if="gridData && gridData.students.length === 0">이 영역에 배정된 학생이 없습니다. 영역(Area) 관리에서 <strong><u>학생
            배정</u></strong> 버튼을 눌러 학생을 배정하세요.
          </template>
          <template v-else-if="gridData && gridData.activities.length === 0">이 영역에 등록된 활동이 없습니다. 영역(Area) 관리에서
            <strong><u>포함할 활동</u></strong>을 추가하세요.
          </template>
          <template v-else>데이터를 불러올 수 없습니다.</template>
        </p>
      </div>

      <!-- 그리드 -->
      <div v-else class="grid-wrapper" @wheel="onGridWheel">
        <table class="grid-table">
          <thead>
          <tr>
            <th
                class="th-fixed th-grade"
                :class="freezeColumns ? 'sticky' : ''"
                style="left: 0"
            >학년
            </th>
            <th
                class="th-fixed th-class"
                :class="freezeColumns ? 'sticky' : ''"
                style="left: 48px"
            >반
            </th>
            <th
                class="th-fixed th-number"
                :class="freezeColumns ? 'sticky' : ''"
                style="left: 96px"
            >번호
            </th>
            <th
                class="th-fixed th-name"
                :class="freezeColumns ? 'sticky' : ''"
                style="left: 144px"
            >이름
            </th>
            <th
                class="th-fixed th-total"
                :class="freezeColumns ? 'sticky' : ''"
                style="left: 244px"
            >합계
            </th>
            <th
                v-for="act in gridData.activities"
                :key="act.id"
                class="th-activity"
                :class="{ 'th-activity--collapsed': collapsedActivities.has(act.id) }"
                :style="collapsedActivities.has(act.id) ? { width: '80px', minWidth: '80px', maxWidth: '80px' } : {}"
                @click="toggleActivity(act.id)"
            >{{ collapsedActivities.has(act.id) ? truncateName(act.name) : act.name }}
            </th>
          </tr>
          </thead>
          <tbody>
          <tr
              v-for="(student, idx) in gridData.students"
              :key="student.id"
              :class="isNewGroup(gridData.students, idx) ? 'row-group-start' : ''"
          >
            <td
                class="td-fixed td-grade"
                :class="[freezeColumns ? 'sticky' : '', isStudentOverLimit(student.id) ? 'td-row--over' : (highlightEmpty && isStudentEmpty(student.id) ? 'td-row--empty' : '')]"
                style="left: 0"
            >{{ student.grade }}
            </td>
            <td
                class="td-fixed td-class"
                :class="[freezeColumns ? 'sticky' : '', isStudentOverLimit(student.id) ? 'td-row--over' : (highlightEmpty && isStudentEmpty(student.id) ? 'td-row--empty' : '')]"
                style="left: 48px"
            >{{ student.class_num }}
            </td>
            <td
                class="td-fixed td-number"
                :class="[freezeColumns ? 'sticky' : '', isStudentOverLimit(student.id) ? 'td-row--over' : (highlightEmpty && isStudentEmpty(student.id) ? 'td-row--empty' : '')]"
                style="left: 96px"
            >{{ student.number }}
            </td>
            <td
                class="td-fixed td-name"
                :class="[freezeColumns ? 'sticky' : '', isStudentOverLimit(student.id) ? 'td-row--over' : (highlightEmpty && isStudentEmpty(student.id) ? 'td-row--empty' : '')]"
                style="left: 144px"
            >{{ student.name }}
            </td>
            <td
                class="td-fixed td-total"
                :class="[
                freezeColumns ? 'sticky' : '',
                isStudentOverLimit(student.id) ? 'td-total--over' : (highlightEmpty && isStudentEmpty(student.id) ? 'td-total--empty' : '')
              ]"
                style="left: 244px"
            >
            <span
                v-if="byteLimit"
                class="total-bytes"
                :class="isStudentOverLimit(student.id) ? 'total-bytes--over' : (highlightEmpty && isStudentEmpty(student.id) ? 'total-bytes--empty' : '')"
            >
              {{ studentTotalBytes(student.id) }} / {{ byteLimit }} Bytes
            </span>
            </td>
            <td
                v-for="act in gridData.activities"
                :key="act.id"
                class="td-cell"
                :style="collapsedActivities.has(act.id) ? { width: '80px', minWidth: '80px', maxWidth: '80px' } : {}"
                :class="{
                'td-cell--collapsed': collapsedActivities.has(act.id),
                'td-cell--saving': getCellSavingState(act.id, student.id) === 'saving',
                'td-cell--saved': getCellSavingState(act.id, student.id) === 'saved',
                'td-cell--error': getCellSavingState(act.id, student.id) === 'error',
                'td-cell--over': isOverLimit(act.id, student.id),
              }"
            >
              <template v-if="!collapsedActivities.has(act.id)">
              <textarea
                  class="cell-input"
                  :class="{ 'cell-input--compact': compactCell }"
                  :value="getCellContent(act.id, student.id)"
                  @input="onCellInput(act.id, student.id, $event)"
                  rows="1"
              />
                <div class="byte-counter" :class="isOverLimit(act.id, student.id) ? 'byte-counter--over' : ''">
                  {{ byteLength(getCellContent(act.id, student.id) || '') }} Bytes
                  <span class="history-sep">|</span>
                  <button class="btn-history" @click.stop="openHistory(act, student)">History</button>
                </div>
              </template>
            </td>
          </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 히스토리 모달 -->
    <CellHistoryModal
        v-if="historyModal"
        :activity-id="historyModal.activityId"
        :student-id="historyModal.studentId"
        :activity-name="historyModal.activityName"
        :student-name="historyModal.studentName"
        @close="historyModal = null"
    />
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
  align-items: flex-start;
  justify-content: space-between;
  flex-wrap: wrap;
  padding: 36px 40px;
  border-bottom: 1px solid #1a2035;
  flex-shrink: 0;
  gap: 16px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.toolbar-right {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.section-wrap {
  height: 100%;
  box-sizing: border-box;
  overflow-y: auto;
}

.section-title {
  font-size: 22px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0 0 6px;
  white-space: nowrap;
}

.section-tips {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 8px;
}

.tip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: #98bcd8;
  background: rgba(26, 32, 53, 0.7);
  border: 1px solid #1a2035;
  border-radius: 6px;
  padding: 3px 10px;
  white-space: nowrap;
}

.area-select {
  padding: 8px 14px;
  border-radius: 10px;
  border: 1px solid #1a2035;
  background-color: #080b14;
  color: #e2e8f0;
  font-size: 15px;
  cursor: pointer;
  outline: none;
  min-width: 180px;
}

.area-select:focus {
  border-color: rgba(59, 91, 219, 0.5);
}

.btn-freeze {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 8px;
  border: 1px solid #1a2035;
  background: none;
  color: #a0bcd8;
  font-size: 14px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  white-space: nowrap;
}

.btn-freeze:hover {
  background-color: #1a2035;
  color: #c8ddf0;
}

.btn-freeze--on {
  color: #a8c8ff;
  border-color: rgba(59, 91, 219, 0.3);
  background-color: rgba(59, 91, 219, 0.08);
}

/* 빈 상태 */
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 48px;
}

.empty-text {
  font-size: 16px;
  color: #a0bcd8;
  margin: 0;
  text-align: center;
  line-height: 1.7;
}

/* 그리드 */
.grid-wrapper {
  flex: 1;
  overflow: auto;
}


.grid-table {
  border-collapse: separate;
  border-spacing: 0;
  min-width: 100%;
}

/* 헤더 */
.grid-table thead tr {
  position: sticky;
  top: 0;
  z-index: 3;
}

.grid-table th {
  font-size: 13px;
  font-weight: 600;
  color: #b0cce0;
  background-color: #080b14;
  padding: 10px 10px;
  border-bottom: 1px solid #1a2035;
  border-right: 1px solid rgba(40, 55, 90, 0.5);
  white-space: nowrap;
  text-align: center;
  letter-spacing: 0.03em;
}

.th-activity {
  width: 320px;
  min-width: 280px;
  cursor: pointer;
  user-select: none;
  white-space: normal;
  word-break: keep-all;
}

.th-activity:hover {
  color: #c8ddf0;
  background-color: #0d1220;
}

.th-activity--collapsed {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  padding: 10px 8px;
  color: #8aaaf8;
}

/* sticky 열 */
.sticky {
  position: sticky;
  z-index: 2;
}

/* 헤더의 sticky 열은 thead의 z-index(3)보다 높아야 스크롤 콘텐츠 위에 유지 */
thead .sticky {
  z-index: 4;
}

.th-fixed,
.td-fixed {
  background-color: #080b14;
}

/* 고정 열 shadow + 틀고정 구분선 — 합계 열이 담당 */
.th-total.sticky,
.td-total.sticky {
  box-shadow: 1px 0 6px rgba(0, 0, 0, 0.4);
  border-right: 1px solid rgba(59, 91, 219, 0.35) !important;
}

/* 데이터 행 */
.grid-table td {
  font-size: 14px;
  color: #dce8f8;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(40, 55, 90, 0.6);
  border-right: 1px solid rgba(40, 55, 90, 0.5);
  vertical-align: top;
  text-align: center;
}

.td-grade, .td-class, .td-number {
  width: 48px;
  min-width: 48px;
  max-width: 48px;
  text-align: center;
  color: #a8c4e0;
  padding: 6px 4px;
}

.th-grade, .th-class, .th-number {
  width: 48px;
  min-width: 48px;
  max-width: 48px;
  text-align: center;
}

.td-name {
  width: 100px;
  min-width: 100px;
  max-width: 100px;
  word-break: break-all;
}

.th-name {
  width: 100px;
  min-width: 100px;
  max-width: 100px;
}

.th-total {
  width: 110px;
  min-width: 110px;
  max-width: 110px;
  text-align: center;
}

.td-total {
  width: 110px;
  min-width: 110px;
  max-width: 110px;
  text-align: center;
  vertical-align: middle;
}

.td-row--over,
.td-total--over {
  background-color: #4a1212 !important;
}

.total-bytes {
  font-size: 12px;
  color: #90b4d4;
}

.total-bytes--over {
  color: #ff9090;
  font-weight: 700;
}

.td-row--empty,
.td-total--empty {
  background-color: #1e1a00 !important;
}

.total-bytes--empty {
  color: #fbbf24;
}

.btn-freeze--warn {
  color: #fbbf24 !important;
  border-color: rgba(251, 191, 36, 0.3) !important;
  background-color: rgba(251, 191, 36, 0.08) !important;
}

/* 반 구분선 */
.row-group-start td {
  border-top: 1px solid rgba(59, 91, 219, 0.3);
}

/* 셀 */
.td-cell {
  padding: 6px 8px;
  width: 600px;
  min-width: 480px;
  position: relative;
  transition: background-color 0.5s ease;
}

.td-cell--saving {
  background-color: rgba(59, 91, 219, 0.3) !important;
}

.td-cell--saved {
  background-color: rgba(52, 211, 153, 0.3) !important;
}

.td-cell--error {
  background-color: rgba(239, 68, 68, 0.4) !important;
  outline: 2px solid rgba(239, 68, 68, 0.8);
}

.td-cell--over {
  background-color: rgba(239, 68, 68, 0.3) !important;
}

.td-cell--collapsed {
  padding: 0;
  background-color: rgba(59, 91, 219, 0.04);
}

.cell-input {
  width: 100%;
  box-sizing: border-box;
  padding: 6px 8px;
  font-size: 16px;
  line-height: 1.5;
  background-color: transparent;
  border: 1px solid rgba(100, 140, 240, 0.5);
  border-radius: 6px;
  color: #e2e8f0;
  resize: none;
  outline: none;
  transition: border-color 0.15s, background-color 0.15s;
  min-height: 60px;
  overflow-y: auto;
}

.cell-input--compact {
  max-height: 60px;
  overflow-y: auto;
}

.cell-input:focus {
  border-color: rgba(226, 232, 240, 0.7);
  background-color: rgba(8, 11, 20, 0.6);
}

.cell-input::placeholder {
  color: var(--clr-text-subtle);
}

.byte-counter {
  font-size: 11px;
  color: var(--clr-text-hint);
  text-align: right;
  padding-top: 2px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 5px;
}

.byte-counter--over {
  color: #f87171;
}

.history-sep {
  color: #2a3a60;
  user-select: none;
}

.btn-history {
  background: none;
  border: none;
  padding: 0;
  font-size: 11px;
  color: #4a6aaa;
  cursor: pointer;
  line-height: 1;
}

.btn-history:hover {
  color: #8aaaf8;
  text-decoration: underline;
}
</style>
