#!/usr/bin/env node
const fs = require('fs/promises')
const path = require('path')

const data = []
const regex = /^#define\s+(\w+)\s+(\w+)/i
const header = path.resolve(__dirname, '../../deltachat-ffi/deltachat.h')

;(async () => {
  console.log('Generating constants...')
  const header_data = await fs.readFile(header, 'utf-8')
  for (const line of header_data.split(/\r?\n/g)) {
    const match = regex.exec(line)
    if (match) {
      const key = match[1]
      const value = parseInt(match[2])
      if (isNaN(value)) continue

      data.push({ key, value })
    }
  }

  delete header_data

  const constants = data
    .filter(
      ({ key }) => key.toUpperCase()[0] === key[0] // check if define name is uppercase
    )
    .sort((lhs, rhs) => {
      if (lhs.key < rhs.key) return -1
      else if (lhs.key > rhs.key) return 1
      return 0
    })
    .map((row) => {
      return `  ${row.key}: ${row.value}`
    })
    .join(',\n')

  const events = data
    .sort((lhs, rhs) => {
      if (lhs.value < rhs.value) return -1
      else if (lhs.value > rhs.value) return 1
      return 0
    })
    .filter((i) => {
      return i.key.startsWith('DC_EVENT_')
    })
    .map((i) => {
      return `  ${i.value}: '${i.key}'`
    })
    .join(',\n')

  // backwards compat
  await fs.writeFile(
    path.resolve(__dirname, '../constants.js'),
    `// Generated!\n\nmodule.exports = {\n${constants}\n}\n`
  )
  // backwards compat
  await fs.writeFile(
    path.resolve(__dirname, '../events.js'),
    `/* eslint-disable quotes */\n// Generated!\n\nmodule.exports = {\n${events}\n}\n`
  )

  await fs.writeFile(
    path.resolve(__dirname, '../lib/constants.ts'),
    `// Generated!\n\nexport enum C {\n${constants.replace(/:/g, ' =')},\n}\n
// Generated!\n\nexport const EventId2EventName: { [key: number]: string } = {\n${events},\n}\n`
  )
})()
