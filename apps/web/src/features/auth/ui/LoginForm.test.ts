import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { cleanup, fireEvent, render, screen } from '@testing-library/vue'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { useSessionStore } from '@/entities/session'
import { createAppI18n } from '@/shared/i18n'

import LoginForm from './LoginForm.vue'

function json(status: number, body: unknown, contentType = 'application/json'): Response {
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': contentType } })
}

const user = { id: '0192f2a0-0000-7000-8000-000000000001', email: 'a@b.co', display_name: 'Ada' }

describe('LoginForm', () => {
  const fetchMock = vi.fn<(request: Request) => Promise<Response>>()

  beforeEach(() => {
    vi.stubGlobal('fetch', fetchMock)
    setActivePinia(createPinia())
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  function renderForm() {
    return render(LoginForm, {
      global: {
        plugins: [createAppI18n('en'), [VueQueryPlugin, { queryClient: new QueryClient() }]],
      },
    })
  }

  async function fill(email: string, password: string): Promise<void> {
    await fireEvent.update(screen.getByLabelText('Email'), email)
    await fireEvent.update(screen.getByLabelText('Password'), password)
    await fireEvent.click(screen.getByRole('button', { name: 'Sign in' }))
  }

  it('validates input before calling the API', async () => {
    renderForm()
    await fill('not-an-email', '')

    expect(await screen.findByText('Enter a valid email address.')).toBeTruthy()
    expect(screen.getByText('This field is required.')).toBeTruthy()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('signs in and emits success', async () => {
    fetchMock.mockResolvedValue(
      json(200, { access_token: 'tok', token_type: 'Bearer', expires_in: 900, user }),
    )
    const view = renderForm()
    await fill('a@b.co', 'correct horse battery')

    await vi.waitFor(() => {
      expect(view.emitted()).toHaveProperty('success')
    })
    expect(useSessionStore().token()).toBe('tok')
    const request = fetchMock.mock.calls[0]?.[0]
    expect(request?.url).toMatch(/\/api\/v1\/auth\/login$/)
  })

  it('shows the translated problem for wrong credentials', async () => {
    fetchMock.mockResolvedValue(
      json(
        401,
        {
          type: 'about:blank',
          title: 'Invalid credentials',
          status: 401,
          code: 'invalid_credentials',
        },
        'application/problem+json',
      ),
    )
    const view = renderForm()
    await fill('a@b.co', 'wrong password!')

    expect((await screen.findByRole('alert')).textContent).toContain('Wrong email or password.')
    expect(view.emitted()).not.toHaveProperty('success')
  })
})
