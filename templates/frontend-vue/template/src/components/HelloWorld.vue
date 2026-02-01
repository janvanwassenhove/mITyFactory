<script setup lang="ts">
defineProps<{
  msg: string
}>()
</script>

<template>
  <div class="hello">
    <h1>{{ msg }}</h1>
    <p>Powered by Vue 3 + TypeScript + Vite</p>
  </div>
</template>

<style scoped>
.hello {
  padding: 2rem;
}

h1 {
  color: #42b883;
  font-size: 2.5rem;
  margin-bottom: 1rem;
}

p {
  color: #666;
}
</style>
