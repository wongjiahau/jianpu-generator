import { defineConfig, devices } from '@playwright/test'
import { defineBddConfig } from 'playwright-bdd'

const testDir = defineBddConfig({
  features: 'e2e/features/**/*.feature',
  steps: 'e2e/features/steps/**/*.ts',
})

export default defineConfig({
  testDir,
  fullyParallel: true,
  // No in-run retries: a flaky test masked here would just report "passed"
  // with no record that it ever failed. Flakiness is instead resolved across
  // whole-suite passes by scripts/resolve-e2e-flakes.mjs (see
  // `test:e2e:resolve`), which reruns only the tests still failing after
  // each pass until the same set fails 3 times in a row.
  retries: 0,
  use: {
    baseURL: 'http://localhost:5173',
    // Needed for Synced Share scenarios: the local `wrangler dev` instance
    // below serves over HTTPS with a self-signed cert (`--local-protocol
    // https`), which every browser rejects by default.
    ignoreHTTPSErrors: true,
  },
  webServer: [
    {
      // Skip `predev` (the cargo-component/jco build) since pkg-component is
      // already built; just start Vite.
      command: 'pnpm exec vite',
      url: 'http://localhost:5173',
      reuseExistingServer: true,
      timeout: 60_000,
      env: {
        // Redirects Synced Share's owner/viewer fetches (both hardcoded to
        // `https://`, see `syncedShareEndpointUrl`/`useSyncedShareViewer`)
        // at the local `live-share-worker` below instead of the real,
        // deployed one — otherwise every Synced Share scenario would burn
        // real Workers KV writes/reads (and its 1,000/day free-tier write
        // quota) on every e2e run.
        VITE_SYNCED_SHARE_HOST: 'localhost:8787',
      },
    },
    {
      // `--local-protocol https` is required: the app code always fetches
      // `https://${host}/...` (see above), and plain `wrangler dev` only
      // serves HTTP. Runs against Miniflare's in-memory KV emulation (no
      // real Cloudflare account involved), so it's free to hit as often as
      // the suite wants and needs no `wrangler login`.
      command: 'npx wrangler dev --local-protocol https --port 8787',
      cwd: '../live-share-worker',
      port: 8787,
      reuseExistingServer: true,
      timeout: 30_000,
    },
  ],
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        launchOptions: {
          args: [
            '--autoplay-policy=no-user-gesture-required',
            // Several tests load real, large assets (soundfonts, PDF fonts).
            // Some sandboxed environments fail to write Chromium's HTTP disk
            // cache for large responses (net::ERR_CACHE_WRITE_FAILURE),
            // which otherwise breaks those fetches entirely. Applied to
            // every test (not just the affected ones) since playwright-bdd's
            // generated spec files don't support a per-scenario
            // `test.use({ launchOptions })` override, and the flags are
            // harmless for tests that don't hit large assets.
            //
            // Each worker runs in its own process, so `process.pid` gives
            // every worker's Chromium instance a distinct cache dir — with
            // `fullyParallel`, workers previously shared one dir and
            // stomped on each other's cache writes/reads, causing
            // intermittent slowdowns and timeouts under parallel runs.
            `--disk-cache-dir=/tmp/chromium-e2e-cache-${process.pid}`,
            '--disable-http-cache',
          ],
        },
      },
    },
  ],
})
