<script setup>
import { Sparkles, Wrench, Bug, AlertTriangle } from 'lucide-vue-next'

defineProps({
  notes: { type: Array, required: true },
})

const emit = defineEmits(['close'])
</script>

<template>
  <div class="modal-overlay">
    <div class="modal-container max-w-[580px] p-8">

      <div class="flex items-center gap-[14px] mb-[22px]">
        <div class="flex items-center justify-center w-[42px] h-[42px] rounded-xl bg-blue/15 border border-blue/30 text-ink-3 shrink-0">
          <Sparkles :size="20" />
        </div>
        <div>
          <h2 class="text-lg font-semibold text-ink m-0 leading-[1.2]">업데이트 완료</h2>
          <p class="text-base text-ink-5 m-0">새로운 버전으로 정상 적용되었습니다.</p>
        </div>
      </div>

      <div v-if="notes.length" class="notes-body flex flex-col gap-4 mb-6 max-h-[300px] overflow-y-auto pr-1">
        <template v-for="(note, index) in notes" :key="note.version">
          <div class="text-lg font-semibold text-ink-3 pb-0.5">
            v{{ note.version }}
            <span class="text-base font-normal text-ink-5 ml-[6px]">{{ note.date }}</span>
          </div>

          <div v-if="note.breaking?.length" class="flex flex-col gap-2">
            <div class="flex items-center gap-[6px] text-lg font-semibold uppercase tracking-[0.05em] text-ink-3">
              <AlertTriangle :size="14" class="shrink-0 text-amber" />
              주요 변경 사항
            </div>
            <ul class="list-disc list-outside pl-[18px] flex flex-col gap-[5px] m-0">
              <li v-for="item in note.breaking" :key="item" class="text-base text-ink-2 leading-[1.5]">{{ item }}</li>
            </ul>
          </div>

          <div v-if="note.features?.length" class="flex flex-col gap-2">
            <div class="flex items-center gap-[6px] text-lg font-semibold uppercase tracking-[0.05em] text-ink-3">
              <Sparkles :size="14" class="shrink-0 text-blue-2" />
              새 기능
            </div>
            <ul class="list-disc list-outside pl-[18px] flex flex-col gap-[5px] m-0">
              <li v-for="item in note.features" :key="item" class="text-base text-ink-2 leading-[1.5]">{{ item }}</li>
            </ul>
          </div>

          <div v-if="note.improvements?.length" class="flex flex-col gap-2">
            <div class="flex items-center gap-[6px] text-lg font-semibold uppercase tracking-[0.05em] text-ink-3">
              <Wrench :size="14" class="shrink-0 text-violet" />
              개선 사항
            </div>
            <ul class="list-disc list-outside pl-[18px] flex flex-col gap-[5px] m-0">
              <li v-for="item in note.improvements" :key="item" class="text-base text-ink-2 leading-[1.5]">{{ item }}</li>
            </ul>
          </div>

          <div v-if="note.bugFixes?.length" class="flex flex-col gap-2">
            <div class="flex items-center gap-[6px] text-lg font-semibold uppercase tracking-[0.05em] text-ink-3">
              <Bug :size="14" class="shrink-0 text-green" />
              버그 수정
            </div>
            <ul class="list-disc list-outside pl-[18px] flex flex-col gap-[5px] m-0">
              <li v-for="item in note.bugFixes" :key="item" class="text-base text-ink-2 leading-[1.5]">{{ item }}</li>
            </ul>
          </div>

          <hr v-if="index < notes.length - 1" class="border-0 border-t border-line-2 my-2 opacity-60" />
        </template>
      </div>

      <div v-else class="text-base text-ink-5 text-center py-5 mb-6">
        이 버전의 릴리즈 노트가 없습니다.
      </div>

      <div class="flex justify-end">
        <button class="btn-primary" @click="emit('close')">확인</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.notes-body::-webkit-scrollbar { width: 4px; }
.notes-body::-webkit-scrollbar-track { background: transparent; }
.notes-body::-webkit-scrollbar-thumb {
  background: var(--c-line-2);
  border-radius: 4px;
}
</style>
