<script setup>
</script>
<template>
  <div class="section-wrap">
    <h2 class="section-title">개요</h2>
    <p class="section-desc">프로젝트 전반적인 현황을 한눈에 확인합니다.</p>
    <div class="placeholder-box">준비 중입니다.</div>
  </div>
</template>

<style scoped>
.section-wrap {
  padding: 36px 40px;
  height: 100%;
  box-sizing: border-box;
  overflow-y: auto;
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
  margin-bottom: 32px;
}

.placeholder-box {
  font-size: 16px;
  color: var(--clr-text-hint);
  border: 1px dashed #1a2035;
  border-radius: 12px;
  padding: 60px;
  text-align: center;
}
</style>
