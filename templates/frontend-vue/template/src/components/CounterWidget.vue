<script setup lang="ts">
import { useCounterStore } from '../stores/counter'

const counter = useCounterStore()
</script>

<template>
  <div class="counter-widget">
    <h2>Counter: {{ counter.count }}</h2>
    <p>Double: {{ counter.doubleCount }}</p>
    <div class="buttons">
      <button @click="counter.decrement">-</button>
      <button @click="counter.increment">+</button>
    </div>
  </div>
</template>

<style scoped>
.counter-widget {
  margin-top: 2rem;
  padding: 1.5rem;
  background: #f5f5f5;
  border-radius: 8px;
  display: inline-block;
}

h2 {
  margin: 0 0 0.5rem;
  color: #333;
}

p {
  color: #666;
  margin-bottom: 1rem;
}

.buttons {
  display: flex;
  gap: 0.5rem;
  justify-content: center;
}

button {
  padding: 0.5rem 1.5rem;
  font-size: 1.25rem;
  border: none;
  border-radius: 4px;
  background: #42b883;
  color: white;
  cursor: pointer;
}

button:hover {
  background: #369970;
}
</style>
