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
  <RouterView />
</template>
