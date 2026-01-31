import React, { useState, useEffect } from 'react'
import * as architecture from 'architecture'
import { createHttpBindings } from '@arinoto/cdk-arch'

const api = architecture.api;
const endpoint = { baseUrl: 'http://localhost:3000' };

const apiClient = createHttpBindings(endpoint, api, ['hello', 'hellos']);

interface HelloEntry {
  name: string;
  when: number;
}

export default function App() {
  const [name, setName] = useState('')
  const [hellos, setHellos] = useState<HelloEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Fetch hellos on component mount
  useEffect(() => {
    fetchHellos()
  }, [])

  const fetchHellos = async () => {
    try {
      setLoading(true)
      setError(null)
      const result = await apiClient.hellos()
      setHellos(result || [])
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch hellos')
      console.error('Failed to fetch hellos:', err)
    } finally {
      setLoading(false)
    }
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return

    try {
      setLoading(true)
      setError(null)
      const result = await apiClient.hello(name)
      console.log('Hello result:', result)
      // Refresh the list
      await fetchHellos()
      setName('')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to submit hello')
      console.error('Failed to submit hello:', err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="app">
      <h1>CDK Arch Web Demo</h1>
      <form onSubmit={handleSubmit} className="hello-form">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter your name"
          disabled={loading}
        />
        <button type="submit" disabled={loading || !name.trim()}>
          {loading ? 'Submitting...' : 'Say Hello'}
        </button>
      </form>

      {error && <div className="error">{error}</div>}

      <div className="hellos-list">
        <h2>Hellos</h2>
        {hellos.length === 0 ? (
          <p>No hellos yet. Submit one above!</p>
        ) : (
          <ul>
            {hellos.map((hello, index) => (
              <li key={index}>Hello, {hello.name}!</li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}