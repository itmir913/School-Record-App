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
  transition: opacity 0.25s ease;
}
.page-fade-enter-from {
  opacity: 0;
}
</style>
