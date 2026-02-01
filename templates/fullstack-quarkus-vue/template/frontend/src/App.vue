<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<template>
  <div id="app">
    <header>
      <nav>
        <router-link to="/">Home</router-link>
        <router-link to="/about">About</router-link>
      </nav>
    </header>
    <main>
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
header {
  background-color: #1a1a2e;
  padding: 1rem 2rem;
  border-bottom: 1px solid #2a2a4a;
}

nav {
  display: flex;
  gap: 1rem;
}

nav a {
  color: #eaeaea;
  text-decoration: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  transition: background-color 0.2s;
}

nav a:hover {
  background-color: #2a2a4a;
}

nav a.router-link-active {
  background-color: #e94560;
  color: white;
}

main {
  padding: 2rem;
}
</style>
