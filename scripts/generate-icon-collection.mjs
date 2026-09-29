import fs from 'node:fs'
import path from 'node:path'
import solar from '@iconify-json/solar/icons.json' with { type: 'json' }
import mdi from '@iconify-json/mdi/icons.json' with { type: 'json' }

const srcDir = path.resolve('src')
const outputDir = path.join(srcDir, 'assets', 'icons')
const collections = { solar, mdi }
const used = { solar: new Set(), mdi: new Set() }

function visit(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) visit(fullPath)
    else if (/\.(vue|ts|tsx|js|json)$/.test(entry.name) && !entry.name.endsWith('-collection.json')) {
      const content = fs.readFileSync(fullPath, 'utf8')
      for (const [, prefix, name] of content.matchAll(/\b(solar|mdi):([a-z0-9_-]+)/g)) used[prefix].add(name)
    }
  }
}
visit(srcDir)

for (const [prefix, source] of Object.entries(collections)) {
  const selected = { prefix, width: source.width, height: source.height, icons: {}, aliases: {} }
  function include(name) {
    if (source.icons[name]) {
      selected.icons[name] = source.icons[name]
    } else if (source.aliases?.[name]) {
      selected.aliases[name] = source.aliases[name]
      include(source.aliases[name].parent)
    } else {
      throw new Error(`Unknown ${prefix} icon: ${name}`)
    }
  }
  for (const name of used[prefix]) include(name)
  fs.mkdirSync(outputDir, { recursive: true })
  fs.writeFileSync(path.join(outputDir, `${prefix}-collection.json`), `${JSON.stringify(selected)}\n`)
  console.log(`Bundled ${used[prefix].size} ${prefix} icons`)
}
