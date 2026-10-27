import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { createAppI18n } from '@/shared/i18n'

import ShareDialog from './ShareDialog.vue'

const FILE = '0192f2a0-0000-7000-8000-0000000000aa'
const BOB = '0192f2a0-0000-7000-8000-0000000000bb'

function json(status: number, body: unknown): Response {
  const type = status >= 400 ? 'application/problem+json' : 'application/json'
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': type } })
}

const bobShare = {
  user_id: BOB,
  email: 'bob@example.com',
  role: 'viewer',
  shared_by: FILE,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
}

describe('ShareDialog', () => {
  const fetchMock = vi.fn<(request: Request) => Promise<Response>>()
  let shares: unknown[] = []
  let posted: unknown

  beforeEach(() => {
    shares = []
    vi.stubGlobal('fetch', fetchMock)
    fetchMock.mockImplementation(async (request) => {
      const { pathname } = new URL(request.url)
      if (request.method === 'GET' && pathname === `/api/v1/files/${FILE}/shares`) {
        return json(200, { items: shares })
      }
      if (request.method === 'POST' && pathname === `/api/v1/files/${FILE}/shares`) {
        const body: unknown = await request.json()
        posted = body
        if (typeof body === 'object' && body !== null && 'email' in body) {
          if (body.email === 'ghost@example.com') {
            return json(404, { status: 404, code: 'user_not_found', title: 'User not found' })
          }
          shares = [bobShare]
          return json(200, bobShare)
        }
      }
      if (request.method === 'DELETE' && pathname === `/api/v1/files/${FILE}/shares/${BOB}`) {
        shares = []
        return new Response(null, { status: 204 })
      }
      return json(500, { status: 500, code: 'internal', title: 'unexpected' })
    })
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  function renderDialog() {
    return render(ShareDialog, {
      props: { fileId: FILE, title: 'Plan', open: true },
      global: {
        plugins: [
          createAppI18n('en'),
          [
            VueQueryPlugin,
            { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) },
          ],
        ],
      },
    })
  }

  async function submit(email: string): Promise<void> {
    await fireEvent.update(screen.getByLabelText('Email address'), email)
    await fireEvent.click(screen.getByRole('button', { name: 'Share' }))
  }

  it('lists people, shares by email and shows the new person', async () => {
    renderDialog()
    expect(await screen.findByText('This file has not been shared with anyone yet.')).toBeTruthy()

    await submit('bob@example.com')

    expect(await screen.findByText('bob@example.com')).toBeTruthy()
    expect(posted).toEqual({ email: 'bob@example.com', role: 'editor' })
  })

  it('validates the email before calling the API', async () => {
    renderDialog()
    await screen.findByText('This file has not been shared with anyone yet.')
    await fireEvent.click(screen.getByRole('button', { name: 'Share' }))

    expect(await screen.findByText('This field is required.')).toBeTruthy()
    expect(fetchMock.mock.calls.some(([r]) => r.method === 'POST')).toBe(false)
  })

  it('shows a translated message when the user does not exist', async () => {
    renderDialog()
    await screen.findByText('This file has not been shared with anyone yet.')
    await submit('ghost@example.com')

    expect(await screen.findByText('No user is registered with this email address.')).toBeTruthy()
  })

  it('removes access', async () => {
    shares = [bobShare]
    renderDialog()
    await fireEvent.click(
      await screen.findByRole('button', { name: 'Remove access for bob@example.com' }),
    )

    await waitFor(() => {
      expect(screen.queryByText('bob@example.com')).toBeNull()
    })
    expect(fetchMock.mock.calls.some(([r]) => r.method === 'DELETE')).toBe(true)
  })
})
