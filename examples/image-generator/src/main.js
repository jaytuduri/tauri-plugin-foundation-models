import { imgAvailability, generateImages } from 'tauri-plugin-apple-intelligence-api'

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

function fillPlaceholder(card, dataBase64, index) {
  card.className = 'img-card'
  const img = document.createElement('img')
  img.src = `data:image/png;base64,${dataBase64}`
  img.alt = `Generated image ${index + 1}`
  const label = document.createElement('div')
  label.className = 'img-label'
  label.textContent = `Image ${index + 1}`
  card.replaceChildren(img, label)
}

async function init() {
  try {
    const status = await imgAvailability()

    if (!status.available) {
      setStatus('error', 'Unavailable')
      showNotice(`Image generation unavailable: ${status.reason}`, true)
      return
    }

    for (const style of status.styles ?? []) {
      const opt = document.createElement('option')
      opt.value = style.id
      opt.textContent = style.name ?? style.id
      styleSelect.appendChild(opt)
    }

    setStatus('ready', 'On-device model ready')
    showNotice('Describe an image and tap Generate. Images appear as they are created.')
    setFormEnabled(true)
    promptEl.focus()
  } catch (err) {
    setStatus('error', 'Error')
    showNotice(`Failed to initialise: ${err}`, true)
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
      { styleId, limit },
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
