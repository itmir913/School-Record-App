<script setup>
import {computed, onMounted, ref, watch} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {Pin, PinOff} from 'lucide-vue-next'
import {useAreaStore} from '../stores/area'

const areaStore = useAreaStore()

const selectedAreaId = ref(null)
const gridData = ref(null)
const loading = ref(false)
const freezeColumns = ref(true)

// 셀별 저장 상태 map: `${activityId}-${studentId}` → 'saving' | 'saved' | null
const savingState = ref(new Map())
// 편집 중인 내용 map
const cellContent = ref(new Map())
// debounce 타이머 map
const debounceTimers = new Map()

onMounted(async () => {
  await areaStore.fetchAreas()
})

watch(selectedAreaId, async (id) => {
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
  } catch (e) {
    console.error(e)
    gridData.value = null
  } finally {
    loading.value = false
  }
})

function cellKey(activityId, studentId) {
  return `${activityId}-${studentId}`
}

function getCellContent(activityId, studentId) {
  return cellContent.value.get(cellKey(activityId, studentId)) ?? ''
}

function getCellSavingState(activityId, studentId) {
  return savingState.value.get(cellKey(activityId, studentId))
}

function onCellInput(activityId, studentId, event) {
  const key = cellKey(activityId, studentId)
  const content = event.target.value
  const map = new Map(cellContent.value)
  map.set(key, content)
  cellContent.value = map

  // debounce 저장
  if (debounceTimers.has(key)) {
    clearTimeout(debounceTimers.get(key))
  }
  const timer = setTimeout(() => saveCell(activityId, studentId, content), 300)
  debounceTimers.set(key, timer)
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
    }, 1200)
  } catch (e) {
    console.error(e)
    const next = new Map(savingState.value)
    next.delete(key)
    savingState.value = next
  }
}

// 바이트 길이 계산 (UTF-8)
function byteLength(str) {
  return new TextEncoder().encode(str).length
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

// 학년+반이 바뀌는 행에 구분선 표시
function isNewGroup(students, index) {
  if (index === 0) return false
  const prev = students[index - 1]
  const curr = students[index]
  return prev.grade !== curr.grade || prev.class_num !== curr.class_num
}
</script>

<template>
  <div class="section">

    <!-- 상단 컨트롤 -->
    <div class="toolbar">
      <div class="toolbar-left">
        <h2 class="section-title">생기부 작성</h2>
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
      </div>

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
    </div>

    <!-- 빈 상태: 영역 미선택 -->
    <div v-if="!selectedAreaId" class="empty-state">
      <p class="empty-text">상단 메뉴에서 작성할 영역을 선택하세요.</p>
    </div>

    <!-- 로딩 -->
    <div v-else-if="loading" class="empty-state">
      <p class="empty-text">불러오는 중...</p>
    </div>

    <!-- 그리드 없음 (학생 또는 활동 없음) -->
    <div v-else-if="!gridData || gridData.students.length === 0 || gridData.activities.length === 0"
         class="empty-state">
      <p class="empty-text">
        <template v-if="gridData && gridData.students.length === 0">이 영역에 배정된 학생이 없습니다. 학생 관리에서 영역에 학생을 배정하세요.
        </template>
        <template v-else-if="gridData && gridData.activities.length === 0">이 영역에 등록된 활동이 없습니다. 영역 편집에서 활동을 추가하세요.
        </template>
        <template v-else>데이터를 불러올 수 없습니다.</template>
      </p>
    </div>

    <!-- 그리드 -->
    <div v-else class="grid-wrapper">
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
              style="left: 224px"
          >합계
          </th>
          <th
              v-for="act in gridData.activities"
              :key="act.id"
              class="th-activity"
          >{{ act.name }}
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
              :class="freezeColumns ? 'sticky' : ''"
              style="left: 0"
          >{{ student.grade }}
          </td>
          <td
              class="td-fixed td-class"
              :class="freezeColumns ? 'sticky' : ''"
              style="left: 48px"
          >{{ student.class_num }}
          </td>
          <td
              class="td-fixed td-number"
              :class="freezeColumns ? 'sticky' : ''"
              style="left: 96px"
          >{{ student.number }}
          </td>
          <td
              class="td-fixed td-name"
              :class="freezeColumns ? 'sticky' : ''"
              style="left: 144px"
          >{{ student.name }}
          </td>
          <td
              class="td-fixed td-total"
              :class="[
                freezeColumns ? 'sticky' : '',
                isStudentOverLimit(student.id) ? 'td-total--over' : ''
              ]"
              style="left: 224px"
          >
            <span
                v-if="byteLimit"
                class="total-bytes"
                :class="isStudentOverLimit(student.id) ? 'total-bytes--over' : ''"
            >
              {{ studentTotalBytes(student.id) }} / {{ byteLimit }}B
            </span>
          </td>
          <td
              v-for="act in gridData.activities"
              :key="act.id"
              class="td-cell"
              :class="{
                'td-cell--saving': getCellSavingState(act.id, student.id) === 'saving',
                'td-cell--saved': getCellSavingState(act.id, student.id) === 'saved',
                'td-cell--over': isOverLimit(act.id, student.id),
              }"
          >
            <textarea
                class="cell-input"
                :value="getCellContent(act.id, student.id)"
                @input="onCellInput(act.id, student.id, $event)"
                rows="3"
            />
            <div
                v-if="getCellContent(act.id, student.id)"
                class="byte-counter"
                :class="isOverLimit(act.id, student.id) ? 'byte-counter--over' : ''"
            >
              {{ byteLength(getCellContent(act.id, student.id)) }}B
            </div>
          </td>
        </tr>
        </tbody>
      </table>
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

.section-title {
  font-size: 22px;
  font-weight: 700;
  color: #e2e8f0;
  margin: 0 0 6px;
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
  color: #5a7aaa;
  font-size: 14px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  white-space: nowrap;
}

.btn-freeze:hover {
  background-color: #1a2035;
  color: #93afd4;
}

.btn-freeze--on {
  color: #7ba8f0;
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
  color: #5a7aaa;
  margin: 0;
  text-align: center;
  line-height: 1.7;
}

/* 그리드 */
.grid-wrapper {
  flex: 1;
  overflow: auto;
}

.grid-wrapper::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.grid-wrapper::-webkit-scrollbar-thumb {
  background-color: #1a2035;
  border-radius: 3px;
}

.grid-table {
  border-collapse: collapse;
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
  color: #5a7aaa;
  background-color: #080b14;
  padding: 10px 14px;
  border-bottom: 1px solid #1a2035;
  white-space: nowrap;
  text-align: left;
  letter-spacing: 0.03em;
}

.th-activity {
  min-width: 220px;
  max-width: 320px;
}

/* sticky 열 */
.sticky {
  position: sticky;
  z-index: 2;
}

.th-fixed.sticky,
.td-fixed.sticky {
  background-color: #080b14;
}

/* 고정 열 shadow — 합계 열이 맡음 */
.th-total.sticky,
.td-total.sticky {
  box-shadow: 2px 0 6px rgba(0, 0, 0, 0.4);
}

/* 데이터 행 */
.grid-table td {
  font-size: 14px;
  color: #c8d8f0;
  padding: 8px 14px;
  border-bottom: 1px solid rgba(26, 32, 53, 0.7);
  vertical-align: top;
}

.td-grade, .td-class, .td-number {
  width: 48px;
  text-align: center;
  color: #7ba3d4;
}

.td-name {
  width: 80px;
  white-space: nowrap;
}

.th-total {
  min-width: 90px;
  text-align: center;
}

.td-total {
  width: 90px;
  text-align: center;
  vertical-align: middle;
}

.td-total--over {
  background-color: rgba(239, 68, 68, 0.06) !important;
}

.total-bytes {
  font-size: 12px;
  color: #5a7aaa;
  white-space: nowrap;
}

.total-bytes--over {
  color: #f87171;
  font-weight: 600;
}

/* 반 구분선 */
.row-group-start td {
  border-top: 2px solid rgba(59, 91, 219, 0.3);
}

/* 셀 */
.td-cell {
  padding: 6px 8px;
  min-width: 220px;
  max-width: 320px;
  position: relative;
}

.td-cell--saving {
  border: 1px solid rgba(59, 91, 219, 0.4) !important;
}

.td-cell--saved {
  border: 1px solid rgba(52, 211, 153, 0.4) !important;
}

.td-cell--over {
  border: 1px solid rgba(239, 68, 68, 0.5) !important;
}

.cell-input {
  width: 100%;
  box-sizing: border-box;
  padding: 6px 8px;
  font-size: 14px;
  line-height: 1.5;
  background-color: transparent;
  border: 1px solid rgba(59, 91, 219, 0.25);
  border-radius: 6px;
  color: #e2e8f0;
  resize: vertical;
  outline: none;
  transition: border-color 0.15s, background-color 0.15s;
  min-height: 72px;
}

.cell-input:focus {
  border-color: rgba(226, 232, 240, 0.7);
  background-color: rgba(8, 11, 20, 0.6);
}

.cell-input::placeholder {
  color: #2a3a58;
}

.byte-counter {
  font-size: 11px;
  color: #3d5580;
  text-align: right;
  padding-top: 2px;
}

.byte-counter--over {
  color: #f87171;
}
</style>
