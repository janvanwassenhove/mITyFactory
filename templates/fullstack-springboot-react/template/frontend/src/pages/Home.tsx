import { useEffect, useState } from 'react'
import axios from 'axios'

const apiUrl = import.meta.env.VITE_API_URL || 'http://localhost:8080'

function Home() {
  const [message, setMessage] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    const fetchData = async () => {
      try {
        const response = await axios.get(`${apiUrl}/api/hello`)
        setMessage(response.data.message)
      } catch (e) {
        setError('Failed to connect to backend')
        console.error(e)
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [])

  return (
    <div className="home">
      <h1>Welcome to {'{{project_name}}'}</h1>
      <div className="card">
        {loading && <p>Connecting to backend...</p>}
        {error && <p className="error">{error}</p>}
        {!loading && !error && <p className="success">{message}</p>}
      </div>
      <div className="features">
        <div className="feature">
          <h3>🚀 Spring Boot Backend</h3>
          <p>Enterprise-ready Java API with Spring Boot 3.2</p>
        </div>
        <div className="feature">
          <h3>⚛️ React Frontend</h3>
          <p>Modern reactive UI with React 18 and TypeScript</p>
        </div>
        <div className="feature">
          <h3>🐳 Docker Ready</h3>
          <p>Containerized deployment with Docker Compose</p>
        </div>
      </div>
      <style>{`
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
      `}</style>
    </div>
  )
}

export default Home
