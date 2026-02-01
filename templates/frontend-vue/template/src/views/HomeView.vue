<script setup lang="ts">
import HelloWorld from '../components/HelloWorld.vue'
import CounterWidget from '../components/CounterWidget.vue'
</script>

<template>
  <div class="home">
    <HelloWorld msg="Welcome to Your Vue.js App" />
    <CounterWidget />
  </div>
</template>

<style scoped>
.home {
  text-align: center;
}
</style>
