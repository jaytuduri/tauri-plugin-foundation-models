import { generateImages, imgAvailability } from 'tauri-plugin-apple-intelligence-api'

const MAX_IMAGES = 4

const dot         = document.getElementById('dot')
const statusLbl   = document.getElementById('status-label')
const gallery     = document.getElementById('gallery')
const notice      = document.getElementById('notice')
const promptEl    = document.getElementById('prompt')
const styleSelect = document.getElementById('style-select')
const countInput  = document.getElementById('count-input')
const generateBtn = document.getElementById('generate-btn')

let busy = false

function setStatus(state, label) {
  dot.className = 'status-dot ' + state
  statusLbl.textContent = label
}

function showNotice(text, isError = false) {
  notice.textContent = text
  notice.className = isError ? 'error' : ''
  notice.style.display = 'block'
}

function hideNotice() {
  notice.style.display = 'none'
}

function setFormEnabled(enabled) {
  promptEl.disabled    = !enabled
  styleSelect.disabled = !enabled
  countInput.disabled  = !enabled
  generateBtn.disabled = !enabled
}

function addPlaceholder() {
  const card = document.createElement('div')
  card.className = 'placeholder-card'
  const spinner = document.createElement('div')
  spinner.className = 'spinner'
  card.appendChild(spinner)
  gallery.appendChild(card)
  return card
}

const SVG_COPY = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>`
const SVG_CHECK = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>`
const SVG_DOWNLOAD = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`

function b64ToBlob(b64) {
  const bin = atob(b64)
  const bytes = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
  return new Blob([bytes], { type: 'image/png' })
}

function flashButton(btn, originalSvg) {
  btn.innerHTML = SVG_CHECK
  setTimeout(() => { btn.innerHTML = originalSvg }, 1500)
}

function makeActionButton(svg, title, onClick) {
  const btn = document.createElement('button')
  btn.className = 'img-action'
  btn.title = title
  btn.innerHTML = svg
  btn.addEventListener('click', onClick)
  return btn
}

function fillPlaceholder(card, dataBase64, index) {
  card.className = 'img-card'
  const img = document.createElement('img')
  img.src = `data:image/png;base64,${dataBase64}`
  img.alt = `Generated image ${index + 1}`
  const blob = b64ToBlob(dataBase64)

  const footer = document.createElement('div')
  footer.className = 'img-footer'

  const copyBtn = makeActionButton(SVG_COPY, 'Copy to clipboard', async () => {
    try {
      await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })])
      flashButton(copyBtn, SVG_COPY)
    } catch { /* clipboard not available */ }
  })

  const label = document.createElement('div')
  label.className = 'img-label'
  label.textContent = `Image ${index + 1}`

  const dlBtn = makeActionButton(SVG_DOWNLOAD, 'Download', () => {
    const a = document.createElement('a')
    a.href = img.src
    a.download = `image-${index + 1}.png`
    a.click()
    flashButton(dlBtn, SVG_DOWNLOAD)
  })

  footer.append(copyBtn, label, dlBtn)
  card.replaceChildren(img, footer)
}

async function init() {
  try {
    const status = await imgAvailability()
    if (!status.available) {
      setStatus('error', `Unavailable: ${status.reason}`)
      showNotice(`Image generation is not available: ${status.reason}`, true)
      return
    }
    for (const style of status.styles) {
      const opt = document.createElement('option')
      opt.value = style.id
      opt.textContent = style.id
      styleSelect.appendChild(opt)
    }
    setStatus('ready', 'On-device model ready')
    showNotice('Describe an image and tap Generate. Images appear as they are created.')
    setFormEnabled(true)
    promptEl.focus()
  } catch (err) {
    setStatus('error', 'Unavailable')
    showNotice(`Failed to check availability: ${err}`, true)
  }
}

async function generate() {
  const text = promptEl.value.trim()
  if (!text || busy) return

  busy = true
  setFormEnabled(false)
  setStatus('busy', 'Generating…')
  hideNotice()
  gallery.replaceChildren()

  const limit   = Math.max(1, Math.min(MAX_IMAGES, parseInt(countInput.value, 10) || 1))
  const styleId = styleSelect.value || undefined

  const placeholders = []
  for (let i = 0; i < limit; i++) {
    placeholders.push(addPlaceholder())
  }

  let received = 0

  try {
    await generateImages(
      [{ type: 'text', value: text }],
      (img) => {
        if (received >= placeholders.length) return
        fillPlaceholder(placeholders[received], img.dataBase64, received)
        received++
      },
      { styleId, limit, creationVariety: limit > 1 ? 'high' : undefined },
    )
  } catch (err) {
    for (let i = received; i < placeholders.length; i++) {
      placeholders[i].remove()
    }
    showNotice(`Generation failed: ${err}`, true)
  } finally {
    busy = false
    setFormEnabled(true)
    setStatus('ready', 'On-device model ready')
  }
}

generateBtn.addEventListener('click', generate)

promptEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    generate()
  }
})

init()
