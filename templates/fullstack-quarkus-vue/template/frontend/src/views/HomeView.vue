<script setup lang="ts">
import { ref, onMounted } from 'vue'
import axios from 'axios'

const message = ref('')
const loading = ref(true)
const error = ref('')

const apiUrl = import.meta.env.VITE_API_URL || 'http://localhost:8080'

onMounted(async () => {
  try {
    const response = await axios.get(`${apiUrl}/api/hello`)
    message.value = response.data.message
  } catch (e) {
    error.value = 'Failed to connect to backend'
    console.error(e)
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="home">
    <h1>Welcome to {{project_name}}</h1>
    <div class="card">
      <p v-if="loading">Connecting to backend...</p>
      <p v-else-if="error" class="error">{{ error }}</p>
      <p v-else class="success">{{ message }}</p>
    </div>
    <div class="features">
      <div class="feature">
        <h3>🚀 Spring Boot Backend</h3>
        <p>Enterprise-ready Java API with Spring Boot 3.2</p>
      </div>
      <div class="feature">
        <h3>💚 Vue.js Frontend</h3>
        <p>Modern reactive UI with Vue 3 and TypeScript</p>
      </div>
      <div class="feature">
        <h3>🐳 Docker Ready</h3>
        <p>Containerized deployment with Docker Compose</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home {
  text-align: center;
  max-width: 800px;
  margin: 0 auto;
}

h1 {
  color: #e94560;
  margin-bottom: 2rem;
}

.card {
  background: #16213e;
  border: 1px solid #2a2a4a;
  border-radius: 8px;
  padding: 2rem;
  margin-bottom: 2rem;
}

.success {
  color: #4ade80;
  font-size: 1.2rem;
}

.error {
  color: #ef4444;
}

.features {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
}

.feature {
  background: #16213e;
  border: 1px solid #2a2a4a;
  border-radius: 8px;
  padding: 1.5rem;
  text-align: left;
}

.feature h3 {
  margin-bottom: 0.5rem;
}

.feature p {
  color: #a0a0a0;
  font-size: 0.9rem;
}
</style>
