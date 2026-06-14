import { cpSync, existsSync, mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const sourceServer = path.join(root, 'server.js')
const targetRoot = path.join(root, '.next', 'standalone')
mkdirSync(targetRoot, { recursive: true })

if (existsSync(sourceServer)) {
  cpSync(sourceServer, path.join(targetRoot, 'server.js'))
} else {
  throw new Error(`workspace wiki web server.js not found at ${sourceServer}`)
}

mkdirSync(path.join(root, '.next', 'static'), { recursive: true })
mkdirSync(path.join(root, 'public'), { recursive: true })
writeFileSync(path.join(root, '.next', 'static', '.keep'), '', 'utf8')
