import { spawn } from 'node:child_process'
import path from 'node:path'
import react from '@vitejs/plugin-react'
import { defineConfig, type Plugin, type ViteDevServer } from 'vite'

const WASM_PACK_ARGS = [
  'build',
  '../crates/jianpu-wasm',
  '--target',
  'web',
  '--out-dir',
  'pkg',
  '--no-opt',
  '--',
  '--features',
  'wav,pdf',
] as const

const WASM_PKG_JS = path.resolve(
  __dirname,
  '../crates/jianpu-wasm/pkg/jianpu_wasm.js',
)

function isRustSource(file: string): boolean {
  return (
    file.endsWith('.rs') ||
    file.endsWith('Cargo.toml') ||
    file.endsWith('Cargo.lock')
  )
}

function wasmDevPlugin(): Plugin {
  let server: ViteDevServer | undefined
  let building = false
  let queued = false
  let debounceTimer: ReturnType<typeof setTimeout> | undefined

  const repoRoot = path.resolve(__dirname, '..')

  function runWasmPack(): Promise<void> {
    const wasmPackBin = path.join(
      __dirname,
      'node_modules',
      '.bin',
      process.platform === 'win32' ? 'wasm-pack.cmd' : 'wasm-pack',
    )

    return new Promise((resolve, reject) => {
      const child = spawn(wasmPackBin, [...WASM_PACK_ARGS], {
        cwd: __dirname,
        stdio: 'inherit',
      })

      child.on('exit', (code) => {
        if (code === 0) {
          resolve()
          return
        }
        reject(new Error(`wasm-pack exited with code ${code ?? 'unknown'}`))
      })
      child.on('error', reject)
    })
  }

  async function rebuild() {
    if (building) {
      queued = true
      return
    }

    building = true
    try {
      console.log('[jianpu-wasm] Rebuilding...')
      await runWasmPack()
      console.log('[jianpu-wasm] Rebuild complete')

      const wasmModule = server?.moduleGraph.getModuleById(WASM_PKG_JS)
      if (wasmModule) {
        server?.moduleGraph.invalidateModule(wasmModule)
      }
      server?.ws.send({ type: 'full-reload' })
    } catch (error) {
      console.error('[jianpu-wasm] Rebuild failed:', error)
    } finally {
      building = false
      if (queued) {
        queued = false
        void rebuild()
      }
    }
  }

  function scheduleRebuild(file: string) {
    if (!isRustSource(file)) {
      return
    }

    clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      void rebuild()
    }, 300)
  }

  return {
    name: 'jianpu-wasm-dev',
    apply: 'serve',
    configureServer(devServer) {
      server = devServer

      devServer.middlewares.use((req, res, next) => {
        if (req.url?.includes('.wasm')) {
          res.setHeader('Cache-Control', 'no-store')
        }
        next()
      })

      const watchPaths = [
        path.join(repoRoot, 'crates/jianpu-wasm/src'),
        path.join(repoRoot, 'src'),
        path.join(repoRoot, 'Cargo.toml'),
        path.join(repoRoot, 'crates/jianpu-wasm/Cargo.toml'),
      ]

      for (const watchPath of watchPaths) {
        devServer.watcher.add(watchPath)
      }

      devServer.watcher.on('change', scheduleRebuild)
      devServer.watcher.on('add', scheduleRebuild)
      devServer.watcher.on('unlink', scheduleRebuild)
    },
  }
}

export default defineConfig({
  base: process.env.VITE_BASE_PATH ?? '/',
  plugins: [react(), wasmDevPlugin()],
  resolve: {
    alias: {
      'jianpu-wasm': path.resolve(
        __dirname,
        '../crates/jianpu-wasm/pkg/jianpu_wasm.js',
      ),
    },
  },
  worker: {
    format: 'es',
  },
  server: {
    fs: {
      allow: ['..'],
    },
  },
})
