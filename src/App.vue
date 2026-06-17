<script setup>
import { onMounted } from 'vue'

onMounted(() => {
  const loader = document.getElementById('app-loading')
  if (!loader) return
  loader.classList.add('fade-out')
  setTimeout(() => loader.remove(), 300)
})
</script>

<template>
  <RouterView v-slot="{ Component }">
    <Transition name="page-fade" mode="out-in">
      <component :is="Component" />
    </Transition>
  </RouterView>
</template>

<style>
.page-fade-enter-active {
  transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1) 0.6s;
}
.page-fade-leave-active {
  transition: opacity 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}
.page-fade-enter-from,
.page-fade-leave-to {
  opacity: 0;
}
</style>
