import React from 'react'
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react'
import App from '../App'
import { api } from 'architecture'

// Check if we should use real API or mocks
const USE_REAL_API = !!process.env.OVERRIDE_BASE_URL 

// Mock hello entry type (matches API response)
interface HelloEntry {
  name: string
  when: number
}

// Shared mock state across tests
let mockHellos: HelloEntry[] = []

// Mock setup for unit tests
function setupMockAPI() {
  mockHellos = []

  const helloHandler = api.getRoute('hello').handler
  const hellosHandler = api.getRoute('hellos').handler

  vi.spyOn(helloHandler, 'invoke').mockImplementation(async (name: string) => {
    mockHellos.push({ name, when: Date.now() })
    return `Hello, ${name}!`
  })

  vi.spyOn(hellosHandler, 'invoke').mockImplementation(async () => {
    return [...mockHellos]
  })
}

// Helper to wait for app to finish loading
async function waitForAppReady() {
  await waitFor(() => {
    expect(screen.getByText('Say Hello')).toBeTruthy()
  }, { timeout: 10000 })
}

describe('App Component', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    if (!USE_REAL_API) {
      setupMockAPI()
    }
  })

  it('renders the app with title', async () => {
    await act(async () => {
      render(<App />)
    })
    expect(screen.getByText('CDK Arch Web Demo')).toBeTruthy()
  })

  it('has input and submit button', async () => {
    await act(async () => {
      render(<App />)
    })
    await waitForAppReady()
    expect(screen.getByPlaceholderText('Enter your name')).toBeTruthy()
    expect(screen.getByText('Say Hello')).toBeTruthy()
  })
})

describe(`App Integration (${USE_REAL_API ? 'Real API' : 'Mocked'})`, () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    if (!USE_REAL_API) {
      setupMockAPI()
    }
  })

  it('allows submitting a hello and clears input', async () => {
    await act(async () => {
      render(<App />)
    })
    await waitForAppReady()

    const input = screen.getByPlaceholderText('Enter your name')
    const button = screen.getByText('Say Hello')

    await act(async () => {
      fireEvent.change(input, { target: { value: 'TestUser' } })
    })

    await act(async () => {
      fireEvent.click(button)
    })

    await waitFor(() => {
      expect(screen.getByText('Say Hello')).toBeTruthy()
      expect((input as HTMLInputElement).value).toBe('')
    }, { timeout: 10000 })
  })

  it('shows submitted hellos in the list', async () => {
    await act(async () => {
      render(<App />)
    })
    await waitForAppReady()

    const input = screen.getByPlaceholderText('Enter your name')
    const button = screen.getByText('Say Hello')

    await act(async () => {
      fireEvent.change(input, { target: { value: 'Alice' } })
    })

    await act(async () => {
      fireEvent.click(button)
    })

    await waitFor(() => {
      const listItems = screen.queryAllByRole('listitem')
      expect(listItems.length).toBeGreaterThan(0)
      expect(listItems.some(item => item.textContent?.includes('Alice'))).toBe(true)
    }, { timeout: 10000 })
  })

  it('can submit multiple hellos', async () => {
    await act(async () => {
      render(<App />)
    })
    await waitForAppReady()

    const input = screen.getByPlaceholderText('Enter your name')
    const button = screen.getByText('Say Hello')

    // Submit first hello
    await act(async () => {
      fireEvent.change(input, { target: { value: 'Bob' } })
    })

    await act(async () => {
      fireEvent.click(button)
    })

    await waitFor(() => {
      const listItems = screen.queryAllByRole('listitem')
      expect(listItems.some(item => item.textContent?.includes('Bob'))).toBe(true)
    }, { timeout: 10000 })

    // Wait for button to be ready again
    await waitForAppReady()

    // Submit second hello
    await act(async () => {
      fireEvent.change(input, { target: { value: 'Carol' } })
    })

    await act(async () => {
      fireEvent.click(button)
    })

    await waitFor(() => {
      const listItems = screen.queryAllByRole('listitem')
      expect(listItems.some(item => item.textContent?.includes('Carol'))).toBe(true)
    }, { timeout: 10000 })
  })

  if (!USE_REAL_API) {
    it('shows error when submission fails', async () => {
      vi.spyOn(api.getRoute('hello').handler, 'invoke').mockRejectedValue(new Error('Test error'))

      await act(async () => {
        render(<App />)
      })
      await waitForAppReady()

      const input = screen.getByPlaceholderText('Enter your name')
      const button = screen.getByText('Say Hello')

      await act(async () => {
        fireEvent.change(input, { target: { value: 'Test' } })
      })

      await act(async () => {
        fireEvent.click(button)
      })

      await waitFor(() => {
        expect(screen.getByText('Test error')).toBeTruthy()
      })
    })
  }
})
