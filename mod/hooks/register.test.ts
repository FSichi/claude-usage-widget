import { expect, mock, test } from 'claude-code/testing'

const PLAN = { status: 'ok', plan: 'Ultra', windows: [] }

const reply = (text: string, isError = false) => ({
  value: { content: [{ type: 'text', text }], isError },
})

/** A session whose app answers with `answer`, recording what reaches the file. */
function session(on: Parameters<Parameters<typeof test>[1]>[1], answer: () => unknown) {
  const written: { path: string; text: string }[] = []
  const clock = mock.clock(on, { now: 1_000_000 })
  mock.env(on, { USERPROFILE: 'C:/home' })
  on('session.start', (_, e) => ({ cwd: e.cwd }))
  on('fs.stat', () => {
    throw new Error('no such file')
  })
  on('fs.write', (_, e) => {
    written.push({ path: e.path, text: e.text })
    return { value: undefined }
  })
  on('mcp.call', answer as never)
  return { clock, written }
}

const start = { cwd: '/', surface: 'desktop', isInteractive: true } as const

test('writes what the app reports, and again every minute', async ($, on) => {
  const { clock, written } = session(on, () => reply(JSON.stringify({ plan: PLAN })))

  await $.session.start(start)
  await clock.advance(1)
  expect(written).toHaveLength(1)
  expect(written[0].path.replaceAll('\\', '/')).toBe('C:/home/.claude-usage-widget/usage.json')
  const snapshot = JSON.parse(written[0].text)
  expect(snapshot.plan).toEqual(PLAN)
  expect(snapshot.updatedAt).toBe(clock.now() - 1)

  await clock.advance(60_000)
  expect(written).toHaveLength(2)
})

test('a reading the app could not make leaves the last one in place', async ($, on) => {
  const unavailable = JSON.stringify({ plan: { status: 'unavailable', windows: [] } })
  const { clock, written } = session(on, () => reply(unavailable))

  await $.session.start(start)
  await clock.advance(60_001)
  expect(written).toHaveLength(0)
})

test('a session without the app writes nothing', async ($, on) => {
  const { clock, written } = session(on, () => {
    throw new Error('no such server')
  })

  await $.session.start(start)
  await clock.advance(60_001)
  expect(written).toHaveLength(0)
})
