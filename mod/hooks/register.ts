import type { Register } from 'claude-code'

const BRIDGE = '0.2.0'
const SERVER = 'ccd_session_mgmt'
const TOOL = 'get_usage'
const PERIOD_MS = 60_000
// Several sessions can run this mod at once; one write per period is enough.
const MIN_GAP_MS = 45_000

export const register: Register = (on) => {
  on('session.start', async ($, e, next) => {
    const base = await $.env.get('LOCALAPPDATA')
    if (!base) return next(e)
    const path = `${base}/ClaudeUsageWidget/usage.json`

    const refresh = async () => {
      try {
        const now = await $.clock.now()
        const stat = await $.fs.stat(path).catch(() => undefined)
        if (stat && now - stat.mtimeMs < MIN_GAP_MS) return

        const result = await $.mcp.call(SERVER, TOOL, {})
        const block = result.content.find((b) => b.type === 'text')
        const text = block && 'text' in block ? block.text : undefined
        if (result.isError || !text) return

        const { plan } = JSON.parse(text)
        if (!plan || plan.status === 'unavailable') return
        await $.fs.write(path, JSON.stringify({ updatedAt: now, bridge: BRIDGE, plan }, null, 2))
      } catch {
        // A terminal session has no such server. The last reading stays, and
        // the widget says so once it grows old.
      }
    }

    void refresh()
    $.clock.every(PERIOD_MS, refresh)
    return next(e)
  })
}
