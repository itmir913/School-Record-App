<script setup>
import {computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch} from 'vue'
import {getCurrentWindow} from '@tauri-apps/api/window'
import {ALargeSmall, ArrowLeftRight, ArrowUpDown, Circle, CircleAlert, ChevronsRight, Eye, EyeOff, Maximize2, Minimize2, Moon, Pin, PinOff, Search, Sun} from '@lucide/vue'
import {useAreaStore} from '../stores/area'
import {useRecordStore} from '../stores/record'
import {useConfigStore} from '../stores/configStore'
import {byteLength} from '../services/recordText'
import CellHistoryModal from '../components/CellHistoryModal.vue'
import QuickReplaceModal from '../components/QuickReplaceModal.vue'

const areaStore = useAreaStore()
const recordStore = useRecordStore()
const configStore = useConfigStore()

const selectedAreaId = ref(null)
const loadError = ref('')
const rootEl = ref(null)

// 툴바 토글은 configStore(APP_CONFIGS)에 저장되어 재진입/재시작 후에도 유지된다.
const freezeColumns = computed(() => configStore.recordToolbar.freezeColumns)
const smartScroll = computed(() => configStore.recordToolbar.smartScroll)
const compactCell = computed(() => configStore.recordToolbar.compactCell)
const highlightEmpty = computed(() => configStore.recordToolbar.highlightEmpty)
const showPreview = computed(() => configStore.recordToolbar.showPreview)
const collapsePersonalInfo = computed(() => configStore.recordToolbar.collapsePersonalInfo)
const collapsedActivities = ref(new Set())

const settingError = ref('')
const copyError = ref('')
const saveError = ref('')

// 토글 저장 실패를 사용자에게 알린다. 저장에 성공했을 때만 true.
async function setToolbarOption(name, value) {
  settingError.value = ''
  try {
    await configStore.setRecordToolbarOption(name, value)
    return true
  } catch (e) {
    settingError.value = `설정을 저장하지 못했습니다: ${e}`
    return false
  }
}

const FONT_SIZE_MIN = 10
const FONT_SIZE_MAX = 28

// setTheme은 저장 실패 시 되돌리고 던진다. 토글 버튼에서 이를 알린다.
async function toggleTheme() {
  settingError.value = ''
  try {
    await configStore.setTheme(configStore.theme === 'dark' ? 'light' : 'dark')
  } catch (e) {
    settingError.value = `설정을 저장하지 못했습니다: ${e}`
  }
}

async function changeFontSize(delta) {
  const next = Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, configStore.recordCellFontSize + delta))
  if (next === configStore.recordCellFontSize) return
  settingError.value = ''
  try {
    await configStore.setRecordCellFontSize(next)
  } catch (e) {
    settingError.value = `설정을 저장하지 못했습니다: ${e}`
  }
}

function toggleActivity(actId) {
  const next = new Set(collapsedActivities.value)
  if (next.has(actId)) next.delete(actId)
  else next.add(actId)
  collapsedActivities.value = next
}

const savingState = ref(new Map())
const cellContent = reactive(new Map())
const debounceTimers = new Map()
// 저장에 실패한 셀과 **그때 저장하려던 내용**.
// 키만 담아두면 재시도할 때 cellContent에서 내용을 찾아야 하는데, 그 사이 그리드가
// 다시 실리면 값이 없어 빈 문자열을 쓰거나(저장된 내용이 지워진다), 같은 활동이 다른
// 영역에도 속해 있으면 엉뚱한 값을 쓴다. 내용을 함께 담아 그 부류를 없앤다.
const failedSaves = new Map()
// 진행 중인 저장. 타이머가 발동한 뒤 결과가 나오기 전까지 그 셀은 debounceTimers에도
// failedSaves에도 없다. 이 공백을 가드와 flush 양쪽에서 함께 봐야 한다 — 한쪽만
// 보면 저장 중인 셀을 두고 창이 닫히거나 전환이 진행된다.
const inFlightSaves = new Map()
const showQuickReplace = ref(false)

// **창을 닫는 경로**에는 onBeforeUnmount가 걸리지 않는다. 섹션 전환은
// <component :is>가 언마운트를 일으키지만, 앱을 닫으면 Vue는 언마운트되지 않고
// 프로세스가 내려간다. 디바운스는 키 입력마다 리셋되므로 한 문장을 쉬지 않고 쓴 뒤
// X를 누르면 그 문장이 통째로 사라졌다. 닫기를 가로채 먼저 저장한다.
let unlistenClose = null
let unmounted = false
// 종료를 막고 경고한 적이 있는가. 두 번째 시도는 통과시켜 갇히지 않게 한다.
let closeWarned = false

// 종료 경고를 푼다. 예전에는 saveCell 성공에서만 풀려서, 한 번 실패한 뒤 영역 전환
// flush로 전부 저장돼 정상으로 돌아와도 경고가 남았다. 그 상태의 다음 X 클릭은
// 저장 시도조차 없이 그대로 종료돼 원래 유실 버그가 되살아났다.
function clearCloseWarning() {
  closeWarned = false
  // 다른 셀이 아직 실패 상태면 그 진단 메시지를 지우면 안 된다.
  // "디스크 가득참"과 "DB 잠김"을 구분할 유일한 단서다.
  if (failedSaves.size === 0) saveError.value = ''
}

onMounted(async () => {
  // await보다 먼저 붙여, 첫 렌더 이후의 폭 변화를 놓치지 않게 한다.
  if (rootEl.value) {
    resizeObserver = new ResizeObserver(onRootResize)
    resizeObserver.observe(rootEl.value)
  }

  const unlisten = await getCurrentWindow().onCloseRequested(async (event) => {
    // 진행 중인 저장을 빼먹으면, 그 셀은 어느 집합에도 없어 가드가 그냥 통과하고
    // Tauri 래퍼가 곧바로 destroy를 불러 진행 중인 쓰기를 끊는다.
    if (debounceTimers.size === 0 && failedSaves.size === 0 && inFlightSaves.size === 0) return

    // 한 번 막고 알린 뒤에도 다시 닫으려 하면 그대로 닫는다.
    // 계속 막으면 디스크가 가득 찬 상황에서 앱을 아예 끌 수 없게 된다
    // (작업 관리자로 강제 종료하면 어차피 내용도 잃는다).
    if (closeWarned) return

    event.preventDefault()
    try {
      await flushPendingDebounces()
    } catch (e) {
      // 저장에 실패했는데 닫아버리면 내용이 사라진다. 한 번은 멈추고 알린다.
      closeWarned = true
      saveError.value =
          `미저장 내용을 저장하지 못했습니다: ${String(e)}
` +
          '내용을 다른 곳에 복사해 두세요. 다시 닫기를 누르면 저장하지 않고 종료합니다.'
      return
    }
    // 저장은 끝났다. 여기서 실패하면 저장 문제가 아니라 창을 닫지 못하는 문제이므로
    // 메시지를 구분한다(예전에는 저장 실패로 잘못 안내했다).
    //
    // destroy에는 core:window:allow-destroy 권한이 필요하다. Tauri의
    // onCloseRequested 래퍼는 preventDefault를 부르지 않은 경우에도 자기가 destroy를
    // 호출하므로, 권한이 없으면 **리스너를 등록했다는 사실만으로 앱이 안 닫힌다.**
    try {
      await getCurrentWindow().destroy()
    } catch (e) {
      // 여기 도달하면 저장은 끝난 뒤다(위 catch에서 이미 return). 경고 래치를
      // 세워도 다음 시도는 가드에서 먼저 빠져나가므로 의미가 없다. 메시지만 남긴다.
      saveError.value = `작업은 저장했지만 창을 닫지 못했습니다: ${String(e)}`
    }
  })

  // 등록은 IPC 왕복이라 await 중에 언마운트될 수 있다. 그러면 onBeforeUnmount는
  // unlistenClose가 아직 null인 것을 보고 지나가고, 리스너만 영영 남는다.
  // 죽은 인스턴스의 핸들러가 쌓이면 각자 preventDefault를 불러 앱이 안 닫힌다.
  if (unmounted) {
    unlisten()
    return
  }
  unlistenClose = unlisten

  try {
    await areaStore.fetchAreas()
  } catch (e) {
    // 영역을 못 불러오면 드롭다운이 비어 "영역이 없음"과 구분되지 않는다.
    loadError.value = `영역 목록을 불러오지 못했습니다: ${String(e)}`
  }
})

onBeforeUnmount(() => {
  unmounted = true
  if (unlistenClose) unlistenClose()
  // 언마운트는 동기라 await할 수 없다. flush를 걸어두면 invoke는 이미 발행되므로
  // 컴포넌트가 사라져도 저장은 끝까지 진행된다(섹션 전환에서 확인된 동작).
  // 이 경로의 실패는 배너를 띄울 컴포넌트가 이미 사라진다. console.error만 남기면
  // 작성한 내용이 아무 표시 없이 사라지므로(CLAUDE.md가 금지하는 silent failure),
  // 컴포넌트보다 오래 사는 store에 담아 워크스페이스가 보여주게 한다.
  flushPendingDebounces().catch(e => {
    recordStore.pendingSaveError =
        `화면을 옮기기 전 저장하지 못한 내용이 있습니다: ${String(e)}`
  })
  stopObservingResize()
})

// 영역 id로 최신 여부를 판별하면 A→B→A 전환에서 구멍이 난다. id가 같아 통과해버려,
// 아주 늦게 도착한 옛 A 응답이 그 사이 입력한 키를 덮어쓴다. 세대 번호로 판별한다.
let gridGen = 0

async function reloadGrid(id) {
  const myGen = ++gridGen
  await recordStore.fetchAreaGrid(id)
  if (myGen !== gridGen) return
  // 늦게 도착한 응답은 store에 반영되지 않는다. 아직 아무것도 실린 적이 없으면
  // gridData가 null이므로, 최신 응답이 도착할 때까지 재구성을 미룬다.
  if (!recordStore.gridData) return
  cellContent.clear()
  // failedSaves는 비우지 않는다. 저장하려던 내용을 자체적으로 들고 있고,
  // 기록은 (활동, 학생)으로 식별되어 어느 영역을 보고 있든 같은 행이므로
  // 그리드가 바뀌어도 그대로 재시도하는 것이 맞다.
  for (const r of recordStore.gridData.records) {
    cellContent.set(cellKey(r.activity_id, r.student_id), r.content)
  }
  savingState.value = new Map()
  await resyncHeights()
}

// 전환 실패로 선택을 되돌릴 때 watch가 다시 도는 것을 막는다.
// 되돌린 값으로 reloadGrid가 돌면 cellContent가 새로 실리며 미저장 내용이 사라진다.
//
// 불리언 가드는 빠른 전환 중 실패와 성공이 겹치면 고착돼, 이후의 정상 전환을
// 조용히 삼킨다. 되돌릴 대상 값을 담아두되 **매 호출에서 무조건 소비**해 고착을 막고,
// 값이 실제로 바뀔 때만 표시를 남겨(안 바뀌면 watch가 안 돌아 표시가 떠돈다) 정상
// 전환을 잘못 삼키지 않게 한다.
// 전환이 끝나기 전에 또 바꾸면 두 호출이 겹쳐, 선택은 C인데 그리드는 A인 상태가
// 남을 수 있다(QuickReplace가 미리보기와 다른 영역에 적용될 수 있다).
const switchingArea = ref(false)

const NO_REVERT = Symbol('no-revert')
let revertTarget = NO_REVERT

watch(selectedAreaId, async (id, prevId) => {
  const isRevert = id === revertTarget
  revertTarget = NO_REVERT
  if (isRevert) return

  switchingArea.value = true
  try {
    await switchArea(id, prevId)
  } finally {
    switchingArea.value = false
  }
})

async function switchArea(id, prevId) {
  // 영역을 바꾸기 전에 미저장 셀을 먼저 저장한다. 실패하면 전환을 되돌린다 —
  // 선택만 바뀌고 그리드는 이전 영역인 상태로 두면, 화면의 영역명과 내용이
  // 어긋나고 QuickReplace가 미리보기와 다른 영역에 적용될 수 있다.
  try {
    await flushPendingDebounces()
  } catch (e) {
    // 그리드를 지우는 loadError가 아니라 배너(saveError)로 알린다.
    // loadError는 그리드를 통째로 대체해, 저장 못 한 내용을 복사할 수도 없게 만든다.
    saveError.value = `이전 영역의 미저장 내용을 저장하지 못했습니다: ${String(e)}`
    const back = prevId ?? null
    // 값이 그대로면 watch가 다시 돌지 않으므로 표시를 남기면 안 된다.
    if (selectedAreaId.value !== back) {
      revertTarget = back
      selectedAreaId.value = back
    }
    return
  }
  loadError.value = ''
  if (!id) {
    recordStore.gridData = null
    return
  }
  try {
    await reloadGrid(id)
    if (selectedAreaId.value !== id) return
    collapsedActivities.value = new Set()
  } catch (e) {
    if (selectedAreaId.value !== id) return
    loadError.value = String(e)
  }
}

// 디바운스 대기 중인 셀을 즉시 DB에 저장한다.
// QuickReplace는 DB를 직접 읽어 교체하므로, 실행 전에 미저장 내용을 먼저 반영해야 한다.
async function flushPendingDebounces() {
  // 1) 대기 중인 타이머를 **먼저** 끈다. 키는 남기므로 아래에서 대상에 들어간다.
  //    나중에 끄면 기다리는 동안 타이머가 발동해 그 셀이 대상에서 빠진다.
  for (const timerId of debounceTimers.values()) clearTimeout(timerId)

  // 2) 진행 중인 저장이 **하나도 없을 때까지** 기다린다. 한 번만 스냅샷을 뜨면
  //    기다리는 사이에 시작된 저장을 놓친다. 타이머는 위에서 껐으므로 새 저장이
  //    무한히 생기지 않는다.
  while (inFlightSaves.size > 0) {
    await Promise.allSettled([...inFlightSaves.values()])
  }

  // 3) 실패분을 먼저 넣고 대기분으로 덮어써, 더 나중에 친 내용이 이기게 한다.
  const targets = new Map(failedSaves)
  for (const key of debounceTimers.keys()) {
    if (cellContent.has(key)) targets.set(key, cellContent.get(key))
  }
  if (targets.size === 0) {
    debounceTimers.clear()
    clearCloseWarning()
    return
  }

  const saves = []
  for (const [key, content] of targets) {
    const [actId, stuId] = key.split('-').map(Number)
    saves.push(recordStore.upsertRecord(actId, stuId, content))
  }
  // clear는 await 성공 후에 실행 — 실패 시 재시도에서 다시 flush 가능하도록
  await Promise.all(saves)
  debounceTimers.clear()
  failedSaves.clear()
  clearCloseWarning()
}

async function handleQuickReplaceDone() {
  showQuickReplace.value = false
  const id = selectedAreaId.value
  if (!id) return
  try {
    await reloadGrid(id)
  } catch (e) {
    if (selectedAreaId.value !== id) return
    loadError.value = String(e)
  }
}

function truncateName(name, max = 10) {
  return name.length > max ? name.slice(0, max) + '…' : name
}

function cellKey(activityId, studentId) {
  return `${activityId}-${studentId}`
}

function getCellContent(activityId, studentId) {
  return cellContent.get(cellKey(activityId, studentId)) ?? ''
}

function normalizeForCopy(str) {
  return str.replace(/[\r\n]+/g, ' ').replace(/ {2,}/g, ' ').trim()
}

function getCellSavingState(activityId, studentId) {
  return savingState.value.get(cellKey(activityId, studentId))
}

function verticalPadding(el) {
  const cs = getComputedStyle(el)
  return parseFloat(cs.paddingTop) + parseFloat(cs.paddingBottom)
}

// 활동 셀에서 textarea를 뺀 나머지 높이 (바이트/Copy 줄 + td 상하 패딩)
function cellOverhead(input) {
  const td = input.closest('td')
  if (!td) return 0
  let extra = verticalPadding(td)
  for (const child of td.children) {
    if (child !== input) extra += child.offsetHeight
  }
  return extra
}

// 내용이 잘리지 않는 최소 높이. box-sizing이 border-box이므로
// scrollHeight(테두리 미포함)에 테두리 높이를 더해야 2px이 잘리지 않는다.
function naturalHeight(el) {
  el.style.height = 'auto'
  return el.scrollHeight + (el.offsetHeight - el.clientHeight)
}

function syncRowHeights(tr) {
  const inputs = Array.from(tr.querySelectorAll('.cell-input'))
  if (!inputs.length) return

  // 1) 모든 셀을 내용에 맞는 자연 높이로 되돌린 뒤 가장 큰 값을 찾는다.
  let target = Math.max(...inputs.map(naturalHeight))

  // 2) 미리보기 열이 켜져 있으면 그 내용까지 담기도록 높이를 키운다.
  //    td는 행 높이만큼 늘어나므로 td.scrollHeight를 재면 이미 늘어난 행 높이를
  //    되먹임해 셀이 실제 필요보다 계속 커진다. 내부 래퍼를 재야 한다.
  const previewInner = tr.querySelector('.preview-inner')
  if (previewInner) {
    const needed = previewInner.offsetHeight
        + verticalPadding(previewInner.parentElement)
        - cellOverhead(inputs[0])
    target = Math.max(target, needed)
  }

  inputs.forEach(el => { el.style.height = target + 'px' })
}

function syncAllRows() {
  document.querySelectorAll('.record-table tr').forEach(syncRowHeights)
}

// 셀높이 고정(compact)일 때는 인라인 높이를 걷어내 CSS의 max-h가 적용되게 한다.
function clearAllRowHeights() {
  document.querySelectorAll('.cell-input').forEach(el => { el.style.height = '' })
}

// 열 구성이나 글자 크기가 바뀌면 줄바꿈 위치가 달라져 기존 높이가 어긋난다.
// 토글 함수마다 호출을 흩뿌리면 빠뜨리기 쉬우므로 상태 변화를 한 곳에서 감시한다.
async function resyncHeights() {
  await nextTick()
  if (compactCell.value) clearAllRowHeights()
  else syncAllRows()
}

watch(
    () => [
      configStore.recordToolbar.freezeColumns,
      configStore.recordToolbar.collapsePersonalInfo,
      configStore.recordToolbar.showPreview,
      configStore.recordToolbar.compactCell,
      configStore.recordCellFontSize,
      collapsedActivities.value,
    ],
    resyncHeights
)

// 창(또는 사이드바) 폭이 바뀌면 줄바꿈 위치가 달라져 기존 셀 높이가 어긋난다.
// 리사이즈 도중에는 매 프레임 콜백이 오므로, 크기 변경이 멈춘 뒤 1회만 재계산한다.
const RESIZE_SETTLE_MS = 150
let resizeObserver = null
let resizeTimer = null
let lastObservedWidth = -1

function onRootResize(entries) {
  const width = entries[0].contentRect.width
  // 세로만 변한 경우(알림 배너 표시 등)는 줄바꿈에 영향이 없다.
  if (width === lastObservedWidth) return
  const isInitialCallback = lastObservedWidth < 0
  lastObservedWidth = width
  // 관찰 시작 직후 오는 첫 콜백은 초기 렌더와 중복이므로 건너뛴다.
  if (isInitialCallback) return

  clearTimeout(resizeTimer)
  resizeTimer = setTimeout(resyncHeights, RESIZE_SETTLE_MS)
}

function stopObservingResize() {
  clearTimeout(resizeTimer)
  resizeTimer = null
  resizeObserver?.disconnect()
  resizeObserver = null
}

async function toggleCompactCell() {
  await setToolbarOption('compactCell', !compactCell.value)
}

function onCellInput(activityId, studentId, event) {
  const key = cellKey(activityId, studentId)
  const content = event.target.value
  cellContent.set(key, content)
  // 새로 친 내용은 아직 한 번도 저장을 시도하지 않았다. 예전 실패 때문에 내려둔
  // 경고를 그대로 두면, 다음 X 클릭이 저장 시도 없이 이 내용을 버리고 종료한다.
  closeWarned = false
  if (!compactCell.value) {
    const tr = event.target.closest('tr')
    if (tr) syncRowHeights(tr)
  }

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
  // 타이머가 발동했으므로 대기 목록에서 뺀다. 남겨두면 이후 flush마다
  // 마운트 이후 편집한 모든 셀을 다시 저장하게 된다.
  debounceTimers.delete(key)
  const stateMap = new Map(savingState.value)
  stateMap.set(key, 'saving')
  savingState.value = stateMap
  const pending = recordStore.upsertRecord(activityId, studentId, content)
  inFlightSaves.set(key, pending)
  try {
    await pending
    const next = new Map(savingState.value)
    next.set(key, 'saved')
    savingState.value = next
    failedSaves.delete(key)
    clearCloseWarning()
    setTimeout(() => {
      const clear = new Map(savingState.value)
      clear.delete(key)
      savingState.value = clear
    }, 500)
  } catch (e) {
    const next = new Map(savingState.value)
    next.set(key, 'error')
    savingState.value = next
    // 타이머는 이미 발동해 사라졌으므로 여기 담아두지 않으면 재시도할 길이 없다.
    // 키만이 아니라 **저장하려던 내용**을 함께 담는다.
    failedSaves.set(key, content)
    // 빨간 셀만으로는 "디스크 가득참"과 "DB 잠김"을 구분할 수 없다.
    // 메시지를 버리지 않고 사용자에게 보여준다.
    saveError.value = `저장하지 못했습니다: ${String(e)}`
  } finally {
    // 그 사이 같은 셀에 새 저장이 시작됐다면 그쪽 것을 지우지 않는다.
    if (inFlightSaves.get(key) === pending) inFlightSaves.delete(key)
  }
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

const totalBytesCache = computed(() => {
  if (!recordStore.gridData) return new Map()
  const map = new Map()
  for (const student of recordStore.gridData.students) {
    let total = 0
    for (const act of recordStore.gridData.activities) {
      total += byteLength(getCellContent(act.id, student.id))
    }
    map.set(student.id, total)
  }
  return map
})

function studentTotalBytes(studentId) {
  return totalBytesCache.value.get(studentId) ?? 0
}

function isStudentOverLimit(studentId) {
  if (!byteLimit.value) return false
  return studentTotalBytes(studentId) > byteLimit.value
}

function isStudentEmpty(studentId) {
  return studentTotalBytes(studentId) === 0
}

function getCellBgClass(actId, studentId) {
  const state = getCellSavingState(actId, studentId)
  if (state === 'saving') return '!bg-blue/30'
  if (state === 'saved') return '!bg-green/30'
  if (state === 'error') return '!bg-red/40 outline outline-2 outline-red/80'
  if (isOverLimit(actId, studentId)) return 'cell-overlimit-bg'
  return ''
}

const copiedCells = ref(new Set())
const copiedStudents = ref(new Set())

function markCopied(setRef, key) {
  setRef.value.add(key)
  setRef.value = new Set(setRef.value)
  setTimeout(() => {
    setRef.value.delete(key)
    setRef.value = new Set(setRef.value)
  }, 1000)
}

async function copyCell(activityId, studentId) {
  try {
    copyError.value = ''
    await navigator.clipboard.writeText(normalizeForCopy(getCellContent(activityId, studentId)))
    markCopied(copiedCells, cellKey(activityId, studentId))
  } catch (e) {
    // 조용히 넘기면 복사된 줄 알고 붙여넣기 때 빈 내용이 들어간다.
    copyError.value = `클립보드 복사에 실패했습니다: ${e}`
  }
}

async function copyStudentRecord(studentId) {
  const joined = recordStore.gridData.activities
    .map(act => normalizeForCopy(getCellContent(act.id, studentId)))
    .filter(c => c !== '')
    .join(configStore.exportCSeparator ?? ' ')
  try {
    copyError.value = ''
    await navigator.clipboard.writeText(joined)
    markCopied(copiedStudents, studentId)
  } catch (e) {
    copyError.value = `클립보드 복사에 실패했습니다: ${e}`
  }
}

const nameColLeft = computed(() => collapsePersonalInfo.value ? 'left-0' : 'left-144px')
const previewColLeft = computed(() => collapsePersonalInfo.value ? 'left-100px' : 'left-244px')
const byteColLeft = computed(() => {
  if (collapsePersonalInfo.value) return showPreview.value ? 'left-460px' : 'left-100px'
  return showPreview.value ? 'left-604px' : 'left-244px'
})

function studentRowBgClass(studentId) {
  if (isStudentOverLimit(studentId)) return 'cell-overlimit'
  if (highlightEmpty.value && isStudentEmpty(studentId)) return 'cell-empty-row'
  return ''
}

const activityColorMap = computed(() => {
  if (!recordStore.gridData) return new Map()
  return new Map(
    recordStore.gridData.activities.map((act, i) => [act.id, i % 12])
  )
})

function getActivityColorClass(actId) {
  const idx = activityColorMap.value.get(actId)
  return idx !== undefined ? `act-hl-${idx}` : ''
}

function studentPreviewSpans(studentId) {
  if (!recordStore.gridData) return []
  return recordStore.gridData.activities
    .map(act => ({ act, content: getCellContent(act.id, studentId) }))
    .filter(s => s.content.trim() !== '')
}

async function togglePreview() {
  if (!await setToolbarOption('showPreview', !showPreview.value)) return
  // 미리보기 열은 셀 높이가 자동일 때만 행 높이가 맞으므로 함께 해제한다.
  if (showPreview.value && compactCell.value) {
    await setToolbarOption('compactCell', false)
  }
}

async function focusActivityCell(actId, studentId) {
  if (collapsedActivities.value.has(actId)) {
    const next = new Set(collapsedActivities.value)
    next.delete(actId)
    collapsedActivities.value = next
    await nextTick()
  }
  const el = document.querySelector(`[data-cell-key="${cellKey(actId, studentId)}"]`)
  if (!el) return
  el.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' })
  el.focus()
}

const historyModal = ref(null)

// 모달의 "현재 버전 저장"은 화면이 아니라 DB의 값을 기록한다. 먼저 저장해 둘을
// 맞추고, 저장에 실패하면 열지 않는다 — 열어 두면 화면 값을 보존했다고 믿지만
// 실제로는 이전 값이 기록된다. 쓰기가 실패하는 상태라 스냅샷도 어차피 실패한다.
async function openHistory(act, student) {
  try {
    await flushPendingDebounces()
  } catch (e) {
    saveError.value =
        `저장하지 못한 내용이 있어 히스토리를 열 수 없습니다: ${String(e)}`
    return
  }
  historyModal.value = {
    activityId: act.id,
    studentId: student.id,
    activityName: act.name,
    studentName: student.name,
    currentContent: getCellContent(act.id, student.id),
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
  <div ref="rootEl" class="h-full" :style="{ '--cell-fs': configStore.recordCellFontSize + 'px' }">
    <div
        class="flex flex-col box-border"
        :class="freezeColumns ? 'h-full overflow-hidden' : ''"
    >

      <!-- 툴바 -->
      <div class="flex flex-wrap items-center px-6 py-2 border-b border-line-2 shrink-0 gap-2 bg-base min-h-15">
        <!-- 너비 상한을 걸지 않는다(CLAUDE.md). 영역 이름은 끝에 구분 정보가 오는
             경우가 많아 자르면 정작 구분할 부분이 사라진다. 이름이 길어 툴바가 두
             줄로 접히는 것은 flex-wrap이 받아내는 의도된 폴백이다. -->
        <div class="flex items-center gap-2 min-w-0">
          <select
              v-model="selectedAreaId"
              :disabled="switchingArea"
              class="py-2.5 px-3.5 rounded-btn border border-line bg-base text-ink text-base cursor-pointer outline-none min-w-[180px] focus:border-blue/50 disabled:cursor-wait disabled:opacity-60"
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
                class="flex items-center justify-center w-[22px] h-[22px] rounded-[5px] border-none bg-transparent text-base text-ink-3 leading-none cursor-pointer transition-[background-color,color] shrink-0 enabled:hover:bg-blue/20 enabled:hover:text-ink-2 disabled:opacity-30 disabled:cursor-default"
                :disabled="configStore.recordCellFontSize <= FONT_SIZE_MIN"
                @click="changeFontSize(-1)"
            >−</button>
            <span class="text-base font-semibold text-blue-2 min-w-8 text-center">{{ configStore.recordCellFontSize }}px</span>
            <button
                class="flex items-center justify-center w-[22px] h-[22px] rounded-[5px] border-none bg-transparent text-base text-ink-3 leading-none cursor-pointer transition-[background-color,color] shrink-0 enabled:hover:bg-blue/20 enabled:hover:text-ink-2 disabled:opacity-30 disabled:cursor-default"
                :disabled="configStore.recordCellFontSize >= FONT_SIZE_MAX"
                @click="changeFontSize(+1)"
            >+</button>
          </div>

          <button
              class="toolbar-btn"
              :class="freezeColumns ? 'text-blue-2 border-blue/30 bg-blue/[0.08]' : 'text-ink-3 border-line'"
              title="틀고정 켜기/끄기"
              @click="setToolbarOption('freezeColumns', !freezeColumns)"
          >
            <Pin v-if="freezeColumns" :size="15"/>
            <PinOff v-else :size="15"/>
            틀고정
          </button>

          <button
              class="toolbar-btn"
              :class="smartScroll ? 'text-blue-2 border-blue/30 bg-blue/[0.08]' : 'text-ink-3 border-line'"
              title="스마트 스크롤: 활동 영역에서 휠 → 좌우 스크롤"
              @click="setToolbarOption('smartScroll', !smartScroll)"
          >
            <ArrowLeftRight v-if="smartScroll" :size="15"/>
            <ArrowUpDown v-else :size="15"/>
            스마트스크롤
          </button>

          <button
              class="toolbar-btn"
              :class="compactCell ? 'text-blue-2 border-blue/30 bg-blue/[0.08]' : 'text-ink-3 border-line'"
              title="셀 높이: 고정(ON) / 자동(OFF)"
              @click="toggleCompactCell"
          >
            <Minimize2 v-if="compactCell" :size="15"/>
            <Maximize2 v-else :size="15"/>
            {{ compactCell ? '셀높이 고정' : '셀높이 자동' }}
          </button>

          <button
              class="toolbar-btn"
              :class="highlightEmpty ? 'text-amber border-amber/30 bg-amber/[0.08]' : 'text-ink-3 border-line'"
              title="기록이 없는 학생 행 강조 켜기/끄기"
              @click="setToolbarOption('highlightEmpty', !highlightEmpty)"
          >
            <CircleAlert v-if="highlightEmpty" :size="15"/>
            <Circle v-else :size="15"/>
            빈 학생
          </button>

          <button
              class="toolbar-btn"
              :class="showPreview ? 'text-blue-2 border-blue/30 bg-blue/[0.08]' : 'text-ink-3 border-line'"
              title="미리보기 열 켜기/끄기"
              @click="togglePreview"
          >
            <Eye v-if="showPreview" :size="15"/>
            <EyeOff v-else :size="15"/>
            미리보기
          </button>

          <button
              class="flex items-center justify-center py-2.5 px-3.5 rounded-lg border bg-transparent cursor-pointer transition-[background-color,color,border-color] text-ink-3 border-line hover:bg-line hover:text-ink-2 disabled:opacity-40 disabled:cursor-not-allowed"
              :disabled="!selectedAreaId || !recordStore.gridData"
              title="빠른 텍스트 교체"
              @click="showQuickReplace = true"
          >
            <Search :size="15"/>
          </button>

          <button
              class="flex items-center justify-center py-2.5 px-3.5 rounded-lg border bg-transparent cursor-pointer transition-[background-color,color,border-color] text-ink-3 border-line hover:bg-line hover:text-ink-2"
              :title="configStore.theme === 'dark' ? '라이트 모드로 전환' : '다크 모드로 전환'"
              @click="toggleTheme"
          >
            <Sun v-if="configStore.theme === 'dark'" :size="15"/>
            <Moon v-else :size="15"/>
          </button>
        </div>
      </div>

      <!-- 셀 저장 실패 알림. 설정 저장 실패와 달리 작성 내용이 걸린 문제이므로
           경고(amber)가 아니라 오류(red)로, 별도 줄에 띄운다. -->
      <div
          v-if="saveError"
          class="px-6 py-2 border-b border-line-2 shrink-0 bg-red/[0.08]"
      >
        <p class="text-base text-red m-0">{{ saveError }}</p>
      </div>

      <!-- 설정 저장/로드·클립보드 복사 실패 알림 -->
      <div
          v-if="settingError || configStore.preferencesError || copyError"
          class="px-6 py-2 border-b border-line-2 shrink-0 bg-amber/[0.08]"
      >
        <p class="text-base text-amber m-0">{{ settingError || configStore.preferencesError || copyError }}</p>
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
        <table class="record-table border-separate border-spacing-0 min-w-full">
          <thead>
          <tr>
            <th
                v-if="!collapsePersonalInfo"
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-[5px] border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-12 min-w-12 max-w-12 left-0 cursor-pointer select-none underline hover:bg-surface"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                title="클릭하여 학년·반·번호 숨기기"
                @click="setToolbarOption('collapsePersonalInfo', true)"
            >학년</th>
            <th
                v-if="!collapsePersonalInfo"
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-[5px] border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-12 min-w-12 max-w-12 left-48px cursor-pointer select-none underline hover:bg-surface"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                title="클릭하여 학년·반·번호 숨기기"
                @click="setToolbarOption('collapsePersonalInfo', true)"
            >반</th>
            <th
                v-if="!collapsePersonalInfo"
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-[5px] border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-12 min-w-12 max-w-12 left-96px cursor-pointer select-none underline hover:bg-surface"
                :class="freezeColumns ? 'sticky top-0 z-[5]' : ''"
                title="클릭하여 학년·반·번호 숨기기"
                @click="setToolbarOption('collapsePersonalInfo', true)"
            >번호</th>
            <th
                class="th-fixed text-[13px] font-semibold bg-base py-2.5 px-2.5 border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-[100px] min-w-[100px] max-w-[100px]"
                :class="[
                  freezeColumns ? 'sticky top-0 z-[5]' : '',
                  nameColLeft,
                  collapsePersonalInfo
                    ? 'text-amber border-l-2 border-l-amber/50 cursor-pointer select-none underline'
                    : 'text-ink-2'
                ]"
                :title="collapsePersonalInfo ? '학년·반·번호 숨김 — 클릭하여 복원' : ''"
                @click="setToolbarOption('collapsePersonalInfo', false)"
            >
              <span class="flex items-center justify-center gap-1">
                <ChevronsRight v-if="collapsePersonalInfo" :size="13" class="shrink-0"/>
                이름
              </span>
            </th>
            <th
                v-if="showPreview"
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-2.5 border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-[360px] min-w-[360px] max-w-[360px]"
                :class="[freezeColumns ? 'sticky top-0 z-[5]' : '', previewColLeft]"
            >미리보기</th>
            <th
                class="th-fixed text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-2.5 border-b border-line border-r border-line whitespace-nowrap text-center tracking-[0.03em] w-[90px] min-w-[90px] max-w-[90px]"
                :class="[freezeColumns ? 'sticky top-0 z-[5] freeze-border-right' : '', byteColLeft]"
            >바이트</th>
            <th
                v-for="act in recordStore.gridData.activities"
                :key="act.id"
                class="text-[13px] font-semibold text-ink-2 bg-base py-2.5 px-2.5 border-b border-line border-r border-line text-center tracking-[0.03em] w-[320px] min-w-[280px] cursor-pointer select-none underline hover:text-ink-2 hover:bg-surface"
                :class="[
                  freezeColumns ? 'sticky top-0 z-[3]' : '',
                  showPreview ? getActivityColorClass(act.id) : '',
                  collapsedActivities.has(act.id)
                    ? 'act-col-collapsed whitespace-nowrap overflow-hidden text-ellipsis !py-2.5 !px-2 text-blue-2'
                    : 'whitespace-normal break-keep'
                ]"
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
                v-if="!collapsePersonalInfo"
                class="td-fixed text-ink-3 bg-base py-1.5 px-1 border-b border-line-2 border-r border-line-2 align-top text-center w-12 min-w-12 max-w-12 left-0"
                :class="[freezeColumns ? 'sticky z-[2]' : '', studentRowBgClass(student.id)]"
            >{{ student.grade }}</td>
            <!-- 반 -->
            <td
                v-if="!collapsePersonalInfo"
                class="td-fixed text-ink-3 bg-base py-1.5 px-1 border-b border-line-2 border-r border-line-2 align-top text-center w-12 min-w-12 max-w-12 left-48px"
                :class="[freezeColumns ? 'sticky z-[2]' : '', studentRowBgClass(student.id)]"
            >{{ student.class_num }}</td>
            <!-- 번호 -->
            <td
                v-if="!collapsePersonalInfo"
                class="td-fixed text-ink-3 bg-base py-1.5 px-1 border-b border-line-2 border-r border-line-2 align-top text-center w-12 min-w-12 max-w-12 left-96px"
                :class="[freezeColumns ? 'sticky z-[2]' : '', studentRowBgClass(student.id)]"
            >{{ student.number }}</td>
            <!-- 이름 -->
            <td
                class="td-fixed text-ink-2 bg-base py-1.5 px-2.5 border-b border-line-2 border-r border-line-2 align-top text-center w-[100px] min-w-[100px] max-w-[100px] break-all"
                :class="[
                  freezeColumns ? 'sticky z-[2]' : '',
                  nameColLeft,
                  collapsePersonalInfo ? 'border-l-2 border-l-amber/40' : '',
                  studentRowBgClass(student.id)
                ]"
            >{{ student.name }}</td>
            <!-- 미리보기 -->
            <td
                v-if="showPreview"
                class="td-fixed bg-base text-ink py-2 px-3 border-b border-line-2 border-r border-line-2 align-top w-[360px] min-w-[360px] max-w-[360px] leading-relaxed overflow-hidden break-all"
                :class="[freezeColumns ? 'sticky z-[2]' : '', previewColLeft, studentRowBgClass(student.id)]"
            >
              <!-- td는 행 높이만큼 늘어나므로, 실제 내용 높이는 이 래퍼로 측정한다 -->
              <div class="preview-inner">
                <template v-for="(seg, i) in studentPreviewSpans(student.id)" :key="seg.act.id">
                  <span v-if="i > 0" class="whitespace-pre-wrap">{{ configStore.exportCSeparator ?? ' ' }}</span>
                  <span
                      class="act-hl-base cursor-pointer hover:opacity-75 transition-opacity duration-100"
                      :class="getActivityColorClass(seg.act.id)"
                      :title="seg.act.name"
                      @click="focusActivityCell(seg.act.id, student.id)"
                  >{{ seg.content }}</span>
                </template>
              </div>
            </td>
            <!-- 바이트 -->
            <td
                class="td-fixed bg-base py-1.5 px-2.5 border-b border-line-2 border-r border-line-2 align-top text-center w-[90px] min-w-[90px] max-w-[90px]"
                :class="[freezeColumns ? 'sticky z-[2] freeze-border-right' : '', byteColLeft, studentRowBgClass(student.id)]"
            >
              <span
                  v-if="byteLimit"
                  class="text-[12px] block leading-tight"
                  :class="isStudentOverLimit(student.id) ? 'text-red font-bold' : (highlightEmpty && isStudentEmpty(student.id) ? 'text-amber' : 'text-ink-3')"
              >{{ studentTotalBytes(student.id) }} / {{ byteLimit }} Bytes</span>
              <button
                  class="bg-transparent border-none px-4 py-2 text-[11px] text-blue-2/70 cursor-pointer leading-none hover:text-blue-2 hover:underline block mx-auto"
                  @click.stop="copyStudentRecord(student.id)"
              >{{ copiedStudents.has(student.id) ? 'Copied!' : 'Copy' }}</button>
            </td>
            <!-- 활동 셀 -->
            <td
                v-for="act in recordStore.gridData.activities"
                :key="act.id"
                class="text-ink-2 py-1.5 px-2 border-b border-line-2 border-r border-line-2 align-top relative transition-[background-color] duration-500 w-[600px] min-w-[480px]"
                :class="[
                  collapsedActivities.has(act.id) ? 'act-col-collapsed !p-0 !bg-blue/[0.04]' : getCellBgClass(act.id, student.id)
                ]"
            >
              <template v-if="!collapsedActivities.has(act.id)">
                <textarea
                    class="cell-input w-full box-border py-1.5 px-2 leading-[1.5] bg-transparent border rounded-[6px] text-ink resize-none outline-none transition-[border-color,background-color] duration-150 min-h-[60px] overflow-y-auto placeholder:text-ink-5"
                    :class="compactCell ? 'max-h-[60px] overflow-y-auto' : ''"
                    :data-cell-key="cellKey(act.id, student.id)"
                    :value="getCellContent(act.id, student.id)"
                    @input="onCellInput(act.id, student.id, $event)"
                    rows="1"
                />
                <div
                    class="text-[11px] text-right pt-0.5 flex items-center justify-end gap-[5px]"
                    :class="isOverLimit(act.id, student.id) ? 'text-red' : 'text-ink-5'"
                >
                  {{ byteLength(getCellContent(act.id, student.id) || '') }} Bytes
                  <span class="text-ink-5 select-none">|</span>
                  <button
                      class="bg-transparent border-none p-0 text-[11px] text-blue-2/70 cursor-pointer leading-none hover:text-blue-2 hover:underline"
                      @click.stop="copyCell(act.id, student.id)"
                  >{{ copiedCells.has(cellKey(act.id, student.id)) ? 'Copied!' : 'Copy' }}</button>
                  <span class="text-ink-5 select-none">|</span>
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
        :current-content="historyModal.currentContent"
        @close="historyModal = null"
    />

    <!-- 빠른 텍스트 교체 모달 -->
    <QuickReplaceModal
        v-if="showQuickReplace && selectedAreaId && recordStore.gridData"
        :area-id="selectedAreaId"
        :grid-data="recordStore.gridData"
        :cell-content="cellContent"
        :flush-pending="flushPendingDebounces"
        @close="showQuickReplace = false"
        @done="handleQuickReplaceDone"
    />
  </div>
</template>

<style scoped>
.record-table { font-size: var(--cell-fs, 14px); }

/* 반 구분선 — 자식 td 선택자 */
.row-group-start td {
  border-top: 1px solid color-mix(in srgb, var(--c-blue) 30%, transparent);
}

.cell-input {
  font-size: calc(var(--cell-fs, 14px) + 2px);
  border-color: var(--c-cell-border);
}
.cell-input:hover {
  border-color: var(--c-line-2);
}
.cell-input:focus {
  border-color: color-mix(in srgb, var(--c-blue) 70%, transparent);
  background-color: var(--c-cell-focus);
}

/* sticky 셀 행 강조 — 불투명 (반투명 bg는 스크롤 시 뒤 내용이 비침) */
.td-fixed.cell-overlimit,
.cell-overlimit-bg {
  background-color: var(--c-overlimit-bg) !important;
}

.td-fixed.cell-empty-row {
  background-color: color-mix(in srgb, var(--c-amber) 18%, var(--c-base)) !important;
}
</style>
