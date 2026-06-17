<script setup>
import {computed, nextTick, onBeforeUnmount, onMounted, ref, watch} from 'vue'
import {ALargeSmall, ArrowLeftRight, CircleAlert, Minimize2, Moon, Pin, PinOff, Sun} from 'lucide-vue-next'
import {useAreaStore} from '../stores/area'
import {useRecordStore} from '../stores/record'
import {useConfigStore} from '../stores/configStore'
import CellHistoryModal from '../components/CellHistoryModal.vue'

const areaStore = useAreaStore()
const recordStore = useRecordStore()
const configStore = useConfigStore()

const selectedAreaId = ref(null)
const loadError = ref('')
const freezeColumns = ref(true)
const smartScroll = ref(true)
const compactCell = ref(true)
const highlightEmpty = ref(false)
const collapsedActivities = ref(new Set())

const FONT_SIZE_MIN = 10
const FONT_SIZE_MAX = 28

async function changeFontSize(delta) {
  const next = Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, configStore.recordCellFontSize + delta))
  if (next === configStore.recordCellFontSize) return
  configStore.setRecordCellFontSize(next)
  if (!compactCell.value) {
    await nextTick()
    document.querySelectorAll('.cell-input').forEach(el => autoResize(el))
  }
}

function toggleActivity(actId) {
  const next = new Set(collapsedActivities.value)
  if (next.has(actId)) next.delete(actId)
  else next.add(actId)
  collapsedActivities.value = next
}

const savingState = ref(new Map())
const cellContent = ref(new Map())
const debounceTimers = new Map()

onMounted(async () => {
  await areaStore.fetchAreas()
})

function clearAllTimers() {
  debounceTimers.forEach(t => clearTimeout(t))
  debounceTimers.clear()
}

onBeforeUnmount(clearAllTimers)

watch(selectedAreaId, async (id) => {
  clearAllTimers()
  loadError.value = ''
  if (!id) {
    recordStore.gridData = null
    return
  }
  try {
    await recordStore.fetchAreaGrid(id)
    if (selectedAreaId.value !== id) return
    const map = new Map()
    for (const r of recordStore.gridData.records) {
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
    if (selectedAreaId.value !== id) return
    loadError.value = String(e)
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

  if (savingState.value.get(key) === 'error') {
    const cleared = new Map(savingState.value)
    cleared.delete(key)
    savingState.value = cleared
  }

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

async function saveCell(activityId, studentId, content) {
  const key = cellKey(activityId, studentId)
  const stateMap = new Map(savingState.value)
  stateMap.set(key, 'saving')
  savingState.value = stateMap
  try {
    await recordStore.upsertRecord(activityId, studentId, content)
    const next = new Map(savingState.value)
    next.set(key, 'saved')
    savingState.value = next
    setTimeout(() => {
      const clear = new Map(savingState.value)
      clear.delete(key)
      savingState.value = clear
    }, 500)
  } catch (e) {
    const next = new Map(savingState.value)
    next.set(key, 'error')
    savingState.value = next
  }
}

function byteLength(str) {
  if (!str) return 0
  const safeStr = String(str)
  const normalizedStr = safeStr.replace(/\r/g, '').replace(/\n/g, '\r\n')
  return new TextEncoder().encode(normalizedStr).length
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
  if (!recordStore.gridData) return 0
  let total = 0
  for (const act of recordStore.gridData.activities) {
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

const copiedCells = ref(new Set())

async function copyCell(activityId, studentId) {
  const content = getCellContent(activityId, studentId)
  await navigator.clipboard.writeText(content)
  const key = cellKey(activityId, studentId)
  copiedCells.value.add(key)
  copiedCells.value = new Set(copiedCells.value)
  setTimeout(() => {
    copiedCells.value.delete(key)
    copiedCells.value = new Set(copiedCells.value)
  }, 1000)
}

const historyModal = ref(null)

function openHistory(act, student) {
  historyModal.value = {
    activityId: act.id,
    studentId: student.id,
    activityName: act.name,
    studentName: student.name,
  }
}

function isNewGroup(students, index) {
  if (index === 0) return false
  const prev = students[index - 1]
  const curr = students[index]
  return prev.grade !== curr.grade || prev.class_num !== curr.class_num
}
</script>

<template>
  <div class="h-full" :style="{ '--cell-fs': configStore.recordCellFontSize + 'px' }">
    <div
        class="flex flex-col box-border"
        :class="freezeColumns ? 'h-full overflow-hidden' : ''"
    >

      <!-- 툴바 -->
      <div class="flex flex-wrap items-center px-6 py-3 border-b border-line shrink-0 gap-2 bg-base">
        <div class="flex items-center gap-2 min-w-0">
          <select
              v-model="selectedAreaId"
              class="py-2 px-3.5 rounded-btn border border-line bg-base text-ink text-sm cursor-pointer outline-none min-w-[180px] focus:border-blue/50"
          >
            <option :value="null" disabled>영역(Area) 선택</option>
            <option v-for="area in areaStore.areas" :key="area.id" :value="area.id">{{ area.name }}</option>
          </select>
        </div>

        <div class="flex items-center flex-wrap justify-end gap-2 ml-auto">
          <!-- 글자 크기 -->
          <div class="flex items-center gap-1 py-2 px-3.5 rounded-lg border border-blue/30 bg-blue/[0.08] text-blue-2" title="셀 글자 크기">
            <ALargeSmall :size="15" class="shrink-0 mr-0.5 opacity-70"/>
            <button
                class="flex items-center justify-center w-[22px] h-[22px] rounded-[5px] border-none bg-transparent text-sm text-ink-3 leading-none cursor-pointer transition-[background-color,color] shrink-0 enabled:hover:bg-white/[0.08] enabled:hover:text-ink-2 disabled:opacity-30 disabled:cursor-default"
                :disabled="configStore.recordCellFontSize <= FONT_SIZE_MIN"
                @click="changeFontSize(-1)"
            >−</button>
            <span class="text-sm font-semibold text-blue-2 min-w-8 text-center">{{ configStore.recordCellFontSize }}px</span>
            <button
                class="flex items-center justify-center w-[22px] h-[22px] rounded-[5px] border-none bg-transparent text-sm text-ink-3 leading-none cursor-pointer transition-[background-color,color] shrink-0 enabled:hover:bg-white/[0.08] enabled:hover:text-ink-2 disabled:opacity-30 disabled:cursor-default"
                :disabled="configStore.recordCellFontSize >= FONT_SIZE_MAX"
                @click="changeFontSize(+1)"
            >+</button>
          </div>

          <button
              class="flex items-center gap-1.5 py-2 px-3.5 rounded-lg border bg-transparent text-sm cursor-pointer transition-[background-color,color,border-color] whitespace-nowrap hover:bg-line hover:text-ink-2"
              :class="freezeColumns ? 'text-blue-2 border-blue/30 bg-blue/[0.08]' : 'text-ink-3 border-line'"
              title="틀고정 켜기/끄기"
              @click="freezeColumns = !freezeColumns"
          >
            <Pin v-if="freezeColumns" :size="15"/>
            <PinOff v-else :size="15"/>
            {{ freezeColumns ? '틀고정 ON' : '틀고정 OFF' }}
          </button>

          <button
              class="flex items-center gap-1.5 py-2 px-3.5 rounded-lg border bg-transparent text-sm cursor-pointer transition-[background-color,color,border-color] whitespace-nowrap hover:bg-line hover:text-ink-2"
              :class="smartScroll ? 'text-blue-2 border-blue/30 bg-blue/[0.08]' : 'text-ink-3 border-line'"
              title="스마트 스크롤: 활동 영역에서 휠 → 좌우 스크롤"
              @click="smartScroll = !smartScroll"
          >
            <ArrowLeftRight :size="15"/>
            {{ smartScroll ? '스마트스크롤 ON' : '스마트스크롤 OFF' }}
          </button>

          <button
              class="flex items-center gap-1.5 py-2 px-3.5 rounded-lg border bg-transparent text-sm cursor-pointer transition-[background-color,color,border-color] whitespace-nowrap hover:bg-line hover:text-ink-2"
              :class="compactCell ? 'text-blue-2 border-blue/30 bg-blue/[0.08]' : 'text-ink-3 border-line'"
              title="셀 높이: 고정(ON) / 자동(OFF)"
              @click="toggleCompactCell"
          >
            <Minimize2 :size="15"/>
            {{ compactCell ? '셀높이 고정' : '셀높이 자동' }}
          </button>

          <button
              class="flex items-center gap-1.5 py-2 px-3.5 rounded-lg border bg-transparent text-sm cursor-pointer transition-[background-color,color,border-color] whitespace-nowrap hover:bg-line hover:text-ink-2"
              :class="highlightEmpty ? 'text-amber border-amber/30 bg-amber/[0.08]' : 'text-ink-3 border-line'"
              title="기록이 없는 학생 행 강조 켜기/끄기"
              @click="highlightEmpty = !highlightEmpty"
          >
            <CircleAlert :size="15"/>
            {{ highlightEmpty ? '빈 학생 ON' : '빈 학생 OFF' }}
          </button>

          <button
              class="flex items-center justify-center w-[34px] h-[34px] rounded-lg border bg-transparent cursor-pointer transition-[background-color,color,border-color] text-ink-3 border-line hover:bg-line hover:text-ink-2"
              :title="configStore.theme === 'dark' ? '라이트 모드로 전환' : '다크 모드로 전환'"
              @click="configStore.setTheme(configStore.theme === 'dark' ? 'light' : 'dark')"
          >
            <Sun v-if="configStore.theme === 'dark'" :size="15"/>
            <Moon v-else :size="15"/>
          </button>
        </div>
      </div>

      <!-- 빈 상태: 영역 미선택 -->
      <div v-if="!selectedAreaId" class="flex items-center justify-center flex-1 min-h-[300px] p-12">
        <p class="text-base text-ink-3 m-0 text-center leading-relaxed">상단 드롭다운 메뉴에서 영역(Area)을 선택하세요.</p>
      </div>

      <!-- 로딩 -->
      <div v-else-if="recordStore.loading" class="flex items-center justify-center flex-1 min-h-[300px] p-12">
        <p class="text-base text-ink-3 m-0 text-center leading-relaxed">불러오는 중...</p>
      </div>

      <!-- 에러 -->
      <div v-else-if="loadError" class="flex items-center justify-center flex-1 min-h-[300px] p-12">
        <p class="text-base text-red m-0 text-center leading-relaxed">{{ loadError }}</p>
      </div>

      <!-- 그리드 없음 -->
      <div
          v-else-if="!recordStore.gridData || recordStore.gridData.students.length === 0 || recordStore.gridData.activities.length === 0"
          class="flex items-center justify-center flex-1 min-h-[300px] p-12"
      >
        <p class="text-base text-ink-3 m-0 text-center leading-relaxed">
          <template v-if="recordStore.gridData && recordStore.gridData.students.length === 0">이 영역에 배정된 학생이 없습니다. 영역(Area) 관리에서 <strong><u>학생 배정</u></strong> 버튼을 눌러 학생을 배정하세요.</template>
          <template v-else-if="recordStore.gridData && recordStore.gridData.activities.length === 0">이 영역에 등록된 활동이 없습니다. 영역(Area) 관리에서 <strong><u>포함할 활동</u></strong>을 추가하세요.</template>
          <template v-else>데이터를 불러올 수 없습니다.</template>
        </p>
      </div>

      <!-- 그리드 -->
      <div
          v-else
          :class="freezeColumns ? 'flex-1 overflow-auto' : 'overflow-x-auto'"
          @wheel="onGridWheel"
      >
        <table class="border-separate border-spacing-0 min-w-full" :style="{ fontSize: 'var(--cell-fs, 14px)' }">
          <thead>
          <tr>
            <th
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-[5px] border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-12 min-w-12 max-w-12"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                style="left: 0"
            >학년</th>
            <th
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-[5px] border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-12 min-w-12 max-w-12"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                style="left: 48px"
            >반</th>
            <th
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-[5px] border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-12 min-w-12 max-w-12"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                style="left: 96px"
            >번호</th>
            <th
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-2.5 border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-[100px] min-w-[100px] max-w-[100px]"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                style="left: 144px"
            >이름</th>
            <th
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-2.5 border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-[110px] min-w-[110px] max-w-[110px]"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                :style="[{ left: '244px' }, freezeColumns ? { borderRight: '1px solid color-mix(in srgb, var(--c-blue) 35%, transparent)' } : {}]"
            >바이트</th>
            <th
                v-for="act in recordStore.gridData.activities"
                :key="act.id"
                class="text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-2.5 border-b border-line border-r border-line text-center tracking-[0.03em] w-[320px] min-w-[280px] cursor-pointer select-none hover:text-ink-2 hover:bg-surface"
                :class="[
                  freezeColumns ? 'sticky top-0 z-[3]' : '',
                  collapsedActivities.has(act.id)
                    ? 'whitespace-nowrap overflow-hidden text-ellipsis !py-2.5 !px-2 text-blue-2'
                    : 'whitespace-normal break-keep'
                ]"
                :style="collapsedActivities.has(act.id) ? { width: '80px', minWidth: '80px', maxWidth: '80px' } : {}"
                @click="toggleActivity(act.id)"
            >{{ collapsedActivities.has(act.id) ? truncateName(act.name) : act.name }}</th>
          </tr>
          </thead>
          <tbody>
          <tr
              v-for="(student, idx) in recordStore.gridData.students"
              :key="student.id"
              :class="isNewGroup(recordStore.gridData.students, idx) ? 'row-group-start' : ''"
          >
            <!-- 학년 -->
            <td
                class="td-fixed text-ink-3 bg-base py-1.5 px-1 border-b border-line-2 border-r border-line-2 align-top text-center w-12 min-w-12 max-w-12"
                :class="[
                  freezeColumns ? 'sticky z-[2]' : '',
                  isStudentOverLimit(student.id) ? '!bg-red/30' : (highlightEmpty && isStudentEmpty(student.id) ? '!bg-amber/[0.18]' : '')
                ]"
                style="left: 0"
            >{{ student.grade }}</td>
            <!-- 반 -->
            <td
                class="td-fixed text-ink-3 bg-base py-1.5 px-1 border-b border-line-2 border-r border-line-2 align-top text-center w-12 min-w-12 max-w-12"
                :class="[
                  freezeColumns ? 'sticky z-[2]' : '',
                  isStudentOverLimit(student.id) ? '!bg-red/30' : (highlightEmpty && isStudentEmpty(student.id) ? '!bg-amber/[0.18]' : '')
                ]"
                style="left: 48px"
            >{{ student.class_num }}</td>
            <!-- 번호 -->
            <td
                class="td-fixed text-ink-3 bg-base py-1.5 px-1 border-b border-line-2 border-r border-line-2 align-top text-center w-12 min-w-12 max-w-12"
                :class="[
                  freezeColumns ? 'sticky z-[2]' : '',
                  isStudentOverLimit(student.id) ? '!bg-red/30' : (highlightEmpty && isStudentEmpty(student.id) ? '!bg-amber/[0.18]' : '')
                ]"
                style="left: 96px"
            >{{ student.number }}</td>
            <!-- 이름 -->
            <td
                class="td-fixed text-ink-2 bg-base py-1.5 px-2.5 border-b border-line-2 border-r border-line-2 align-top text-center w-[100px] min-w-[100px] max-w-[100px] break-all"
                :class="[
                  freezeColumns ? 'sticky z-[2]' : '',
                  isStudentOverLimit(student.id) ? '!bg-red/30' : (highlightEmpty && isStudentEmpty(student.id) ? '!bg-amber/[0.18]' : '')
                ]"
                style="left: 144px"
            >{{ student.name }}</td>
            <!-- 바이트 -->
            <td
                class="td-fixed bg-base py-1.5 px-2.5 border-b border-line-2 border-r border-line-2 align-top text-center w-[110px] min-w-[110px] max-w-[110px]"
                :class="[
                  freezeColumns ? 'sticky z-[2]' : '',
                  isStudentOverLimit(student.id) ? '!bg-red/30' : (highlightEmpty && isStudentEmpty(student.id) ? '!bg-amber/[0.18]' : '')
                ]"
                :style="[
                  { left: '244px' },
                  freezeColumns ? { borderRight: '1px solid color-mix(in srgb, var(--c-blue) 35%, transparent)' } : {}
                ]"
            >
              <span
                  v-if="byteLimit"
                  class="text-[12px]"
                  :class="isStudentOverLimit(student.id) ? 'text-red font-bold' : (highlightEmpty && isStudentEmpty(student.id) ? 'text-amber' : 'text-ink-3')"
              >{{ studentTotalBytes(student.id) }} / {{ byteLimit }} Bytes</span>
            </td>
            <!-- 활동 셀 -->
            <td
                v-for="act in recordStore.gridData.activities"
                :key="act.id"
                class="text-ink-2 py-1.5 px-2 border-b border-line-2 border-r border-line-2 align-top relative transition-[background-color] duration-500 w-[600px] min-w-[480px]"
                :class="{
                  '!p-0 !bg-blue/[0.04]': collapsedActivities.has(act.id),
                  '!bg-blue/30': !collapsedActivities.has(act.id) && getCellSavingState(act.id, student.id) === 'saving',
                  '!bg-green/30': !collapsedActivities.has(act.id) && getCellSavingState(act.id, student.id) === 'saved',
                  '!bg-red/40 outline outline-2 outline-red/80': !collapsedActivities.has(act.id) && getCellSavingState(act.id, student.id) === 'error',
                  '!bg-red/30': !collapsedActivities.has(act.id) && isOverLimit(act.id, student.id),
                }"
                :style="collapsedActivities.has(act.id) ? { width: '80px', minWidth: '80px', maxWidth: '80px' } : {}"
            >
              <template v-if="!collapsedActivities.has(act.id)">
                <textarea
                    class="cell-input w-full box-border py-1.5 px-2 leading-[1.5] bg-transparent border rounded-[6px] text-ink resize-none outline-none transition-[border-color,background-color] duration-150 min-h-[60px] overflow-y-auto placeholder:text-ink-5"
                    :class="compactCell ? 'max-h-[60px] overflow-y-auto' : ''"
                    :style="{ fontSize: 'calc(var(--cell-fs, 14px) + 2px)' }"
                    :value="getCellContent(act.id, student.id)"
                    @input="onCellInput(act.id, student.id, $event)"
                    rows="1"
                />
                <div
                    class="text-[11px] text-right pt-0.5 flex items-center justify-end gap-[5px]"
                    :class="isOverLimit(act.id, student.id) ? 'text-red' : 'text-ink-5'"
                >
                  {{ byteLength(getCellContent(act.id, student.id) || '') }} Bytes
                  <span class="text-line-2 select-none">|</span>
                  <button
                      class="bg-transparent border-none p-0 text-[11px] text-blue-2/70 cursor-pointer leading-none hover:text-blue-2 hover:underline"
                      @click.stop="copyCell(act.id, student.id)"
                  >{{ copiedCells.has(cellKey(act.id, student.id)) ? 'Copied!' : 'Copy' }}</button>
                  <span class="text-line-2 select-none">|</span>
                  <button
                      class="bg-transparent border-none p-0 text-[11px] text-blue-2/70 cursor-pointer leading-none hover:text-blue-2 hover:underline"
                      @click.stop="openHistory(act, student)"
                  >History</button>
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
/* 반 구분선 — 자식 td 선택자 */
.row-group-start td {
  border-top: 1px solid color-mix(in srgb, var(--c-blue) 30%, transparent);
}

.cell-input {
  border-color: var(--c-cell-border);
}
.cell-input:hover {
  border-color: var(--c-line-2);
}
.cell-input:focus {
  border-color: color-mix(in srgb, var(--c-blue) 70%, transparent);
  background-color: var(--c-cell-focus);
}
</style>
