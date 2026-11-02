import type { Register } from 'claude-code'

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
      const now = await $.clock.now()
      try {
        const stat = await $.fs.stat(path).catch(() => undefined)
        if (stat && now - stat.mtimeMs < MIN_GAP_MS) return

        const result = await $.mcp.call(SERVER, TOOL, {})
        const block = result.content.find((b) => b.type === 'text')
        const text = block && 'text' in block ? block.text : undefined
        if (result.isError || !text) throw new Error(text ?? 'empty result')

        const { plan } = JSON.parse(text)
        await $.fs.write(path, JSON.stringify({ updatedAt: now, plan }, null, 2))
      } catch (error) {
        await $.fs.write(path, JSON.stringify({ updatedAt: now, error: String(error) }, null, 2))
      }
    }

    void refresh()
    $.clock.every(PERIOD_MS, refresh)
    return next(e)
  })
}
