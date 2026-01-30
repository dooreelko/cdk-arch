// Vitest setup file
import { expect } from 'vitest'
import { JSDOM } from 'jsdom'

// Set up JSDOM
const dom = new JSDOM('<!DOCTYPE html><html><body><div id="root"></div></body></html>', {
  url: 'http://localhost:3002',
  pretendToBeVisual: true,
})
global.window = dom.window as any
global.document = dom.window.document
global.navigator = dom.window.navigator

// Use native Node fetch for real API tests (Node 18+)
// For mock mode, fetch is never actually called since we mock handler.invoke()
global.fetch = globalThis.fetch
global.Request = globalThis.Request ?? dom.window.Request
global.Response = globalThis.Response ?? dom.window.Response
